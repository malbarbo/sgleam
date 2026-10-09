//! The svg of an `Image` of `sgleam/image`. The image is a value of the
//! compiled JavaScript, and this module reads it into a sinteract [`Scene`],
//! which the svg renderer of sinteract writes out.
//!
//! The size of a text comes from here too, so the text measures with the
//! face that draws it.

use std::sync::atomic::{AtomicUsize, Ordering};

use rquickjs::class::{JsClass, Readable, Trace, Tracer};
use rquickjs::function::This;
use rquickjs::{
    Class, Constructor, Ctx, Exception, FromJs, Function, JsLifetime, Object, Result, Value,
    convert::List,
};
use sinteract::asset::decode_data_uri;
use sinteract::renderer::Renderer;
use sinteract::renderer::svg::SvgRenderer;
use sinteract::scene::{
    Bitmap, ClipPath, Dash, FillRule, FontStyle, Image, LineCap, LineJoin, Paint, Path,
    PathBuilder, PathStyle, Sampling, Scene, Text, TextSpec,
};

/// The number in the id prefix of the next svg. The page puts every svg of a
/// run into one HTML document, so two svgs must not share an id.
static NEXT_SVG: AtomicUsize = AtomicUsize::new(0);

/// How far from a whole number, or from 0, a coordinate may be and still
/// count as one.
const FP_EPSILON: f64 = 1.0e-10;

/// The svg of `image` on a canvas of `width` by `height`.
pub fn to_svg<'js>(image: Object<'js>, width: f64, height: f64) -> Result<String> {
    let mut scene = Scene::new(width as f32, height as f32);
    draw(&image, &mut scene)?;
    let prefix = format!("s{}-", NEXT_SVG.fetch_add(1, Ordering::Relaxed));
    let mut renderer = SvgRenderer::with_id_prefix(&prefix).expect("a valid id prefix");
    let Ok(svg) = renderer.render(&scene);
    Ok(svg.to_owned())
}

/// Puts `image` into `scene`. A crop inside another becomes one clip of the
/// two boxes together, so no clip of the scene holds another, and the
/// nesting limit of sinteract drops nothing.
fn draw(image: &Object, scene: &mut Scene) -> Result<()> {
    let mut runs = Vec::new();
    walk(image, &None, &mut runs)?;
    for (region, leaves) in runs {
        let Some(region) = region else {
            leaves.into_iter().for_each(|leaf| leaf.add_to(scene));
            continue;
        };
        let Some((&(x, y), rest)) = region.split_first() else {
            continue;
        };
        let mut clip = ClipPath::builder(FillRule::default(), x as f32, y as f32);
        for &(x, y) in rest {
            clip = clip.line_to(x as f32, y as f32);
        }
        scene.clip(clip.build(), |inner| {
            leaves.into_iter().for_each(|leaf| leaf.add_to(inner));
        });
    }
    Ok(())
}

/// The area that the crops around an element leave visible, as the corners
/// of a convex polygon. `None` is no crop, and no corners is an area with
/// nothing in it.
type Region = Option<Vec<(f64, f64)>>;

/// An element of the scene, before it goes into a clip.
enum Leaf {
    Path(Path),
    Text(Text),
    Bitmap(Bitmap),
}

impl Leaf {
    fn add_to(self, scene: &mut Scene) {
        match self {
            Leaf::Path(path) => scene.add_path(path),
            Leaf::Text(text) => scene.add_text(text),
            Leaf::Bitmap(bitmap) => scene.add_bitmap(bitmap),
        }
    }
}

/// Appends the elements of `image` to `runs`, which groups the elements in a
/// row that share a region, in drawing order.
fn walk(image: &Object, region: &Region, runs: &mut Vec<(Region, Vec<Leaf>)>) -> Result<()> {
    let leaf = match class(image)?.as_str() {
        "Path" => path(image)?.map(Leaf::Path),
        "Combination" => {
            walk(&image.get("0")?, region, runs)?;
            walk(&image.get("1")?, region, runs)?;
            None
        }
        "Crop" => {
            let corners = corners(affine(&image.get("box")?)?);
            let inside = intersect(region, &corners);
            walk(&image.get("image")?, &inside, runs)?;
            None
        }
        "Text" => text(image)?.map(Leaf::Text),
        "Bitmap" => {
            let transform = affine(&image.get("box")?)?;
            let loaded: MaybeLoaded = image.get("image")?;
            loaded.map(|loaded| {
                let image = loaded.borrow().0.clone();
                Leaf::Bitmap(Bitmap {
                    image,
                    transform,
                    sampling: Sampling::Smooth,
                })
            })
        }
        other => return Err(type_error(image.ctx(), &format!("cannot draw a {other}"))),
    };
    let Some(leaf) = leaf else {
        return Ok(());
    };
    if region.as_ref().is_some_and(Vec::is_empty) {
        return Ok(());
    }
    match runs.last_mut() {
        Some((last, leaves)) if last == region => leaves.push(leaf),
        _ => runs.push((region.clone(), vec![leaf])),
    }
    Ok(())
}

/// The corners of the box of `m`, in the convention of
/// [`ClipPath::from_transform`].
fn corners(m: [f32; 6]) -> [(f64, f64); 4] {
    let [a, b, c, d, e, f] = m.map(f64::from);
    [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
        .map(|(u, v)| (a * u + c * v + e, b * u + d * v + f))
}

/// How far a point may be outside a box and still count as inside, so a box
/// that matches the region up to rounding does not cut it.
const INSIDE_TOLERANCE: f64 = 1.0e-3;

/// The part of `region` inside the box with `corners`. A box with no area, or
/// one that is not finite, leaves nothing.
fn intersect(region: &Region, corners: &[(f64, f64); 4]) -> Region {
    let finite = corners.iter().all(|&(x, y)| x.is_finite() && y.is_finite());
    let area = cross(corners[0], corners[1], corners[2]);
    if !finite || area == 0.0 {
        return Some(Vec::new());
    }
    // The sign makes the distance to an edge positive inside the box,
    // whichever way its corners turn.
    let sign = area.signum();
    let edges = || {
        corners
            .iter()
            .copied()
            .zip(corners.iter().copied().cycle().skip(1))
    };
    let distance = |(p, q): ((f64, f64), (f64, f64)), r: (f64, f64)| {
        sign * cross(p, q, r) / (q.0 - p.0).hypot(q.1 - p.1)
    };
    let Some(region) = region else {
        return Some(corners.to_vec());
    };
    let contains = |r: (f64, f64)| edges().all(|e| distance(e, r) >= -INSIDE_TOLERANCE);
    if region.iter().all(|&r| contains(r)) {
        return Some(region.clone());
    }
    // Sutherland-Hodgman. The box is convex, so each of its edges cuts away
    // what lies outside it.
    let mut out = region.clone();
    for edge in edges() {
        let points = std::mem::take(&mut out);
        let sides = points
            .iter()
            .copied()
            .zip(points.iter().copied().cycle().skip(1));
        for (p, q) in sides {
            let (dp, dq) = (distance(edge, p), distance(edge, q));
            if dp >= 0.0 {
                out.push(p);
            }
            if (dp >= 0.0) != (dq >= 0.0) {
                let t = dp / (dp - dq);
                out.push((p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)));
            }
        }
        // A corner on the edge comes back as itself and as the point where
        // the edge cuts the side.
        let same = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).hypot(p.1 - q.1) < INSIDE_TOLERANCE;
        out.dedup_by(|p, q| same(*p, *q));
        while let [first, .., last] = out[..]
            && same(first, last)
        {
            out.pop();
        }
    }
    if out.len() < 3 {
        out.clear();
    }
    Some(out)
}

/// Twice the signed area of the triangle `p`, `q`, `r`.
fn cross(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> f64 {
    (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
}

/// The image of a `Bitmap`, so a frame draws the image with no decode. The
/// garbage collector of JavaScript frees the image with the last bitmap that
/// holds it.
pub struct Loaded(Image);

unsafe impl<'js> JsLifetime<'js> for Loaded {
    type Changed<'to> = Loaded;
}

impl<'js> Trace<'js> for Loaded {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

impl<'js> JsClass<'js> for Loaded {
    const NAME: &'static str = "Loaded";

    type Mutable = Readable;

    /// The prototype has the `equals` that the `==` of Gleam calls. Two
    /// images are equal when their files are, as sinteract compares them.
    /// Without it, `==` would find no field to compare and take any two
    /// bitmaps as equal.
    fn prototype(ctx: &Ctx<'js>) -> Result<Option<Object<'js>>> {
        let proto = Object::new(ctx.clone())?;
        let equals = |this: This<Class<'js, Loaded>>, other: Value<'js>| {
            other
                .as_object()
                .and_then(Class::<Loaded>::from_object)
                .is_some_and(|other| this.borrow().0 == other.borrow().0)
        };
        proto.set("equals", Function::new(ctx.clone(), equals)?)?;
        Ok(Some(proto))
    }

    fn constructor(_ctx: &Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        Ok(None)
    }
}

/// The image of a bitmap, or `None` when it did not load.
type MaybeLoaded<'js> = Option<Class<'js, Loaded>>;

/// `system.load_bitmap` gives the width and the height of the image file at
/// `path`, and its image. A file that does not load gives no size and no
/// image. An image larger than the limit of sinteract shrinks, and the size
/// stays the one of the original.
pub fn load_bitmap<'js>(ctx: Ctx<'js>, path: String) -> Result<List<(f64, f64, MaybeLoaded<'js>)>> {
    let Some(blob) = crate::host::load_bitmap(&path) else {
        return Ok(List((0.0, 0.0, None)));
    };
    match Image::load_with_head(blob) {
        Ok((image, head)) if head.width > 0 && head.height > 0 => Ok(List((
            f64::from(head.width),
            f64::from(head.height),
            Some(Class::instance(ctx, Loaded(image))?),
        ))),
        Ok(_) => {
            eprintln!("Error: {path} has no pixels");
            Ok(List((0.0, 0.0, None)))
        }
        Err(err) => {
            eprintln!("Error reading {path}: {err}");
            Ok(List((0.0, 0.0, None)))
        }
    }
}

/// `system.load_image` gives the image in `data_uri`, or no image. An image
/// larger than the limit of sinteract shrinks.
pub fn load_image<'js>(ctx: Ctx<'js>, data_uri: String) -> Result<MaybeLoaded<'js>> {
    match decode_data_uri(&data_uri).and_then(|blob| Image::load(blob).ok()) {
        Some(image) => Ok(Some(Class::instance(ctx, Loaded(image))?)),
        None => Ok(None),
    }
}

/// The path of a `Path`, or `None` when its commands do not start with a
/// move, which svg does not draw either.
fn path(image: &Object) -> Result<Option<Path>> {
    let style = Style::read(&image.get("style")?)?;
    let commands: Vec<Object> = list(image.get("commands")?)?;
    let Some((first, rest)) = commands.split_first() else {
        return Ok(None);
    };
    if class(first)? != "MoveTo" {
        return Ok(None);
    }
    // A thin outline on a half pixel covers one row of pixels, and on a whole
    // pixel it blurs over two. The epsilon takes 9.999999999999998 as 10.
    let snap = |v: f64| {
        if style.aligned() {
            (v + FP_EPSILON).floor() + 0.5
        } else {
            v
        }
    };
    let point = |p: Object| -> Result<(f32, f32)> {
        Ok((snap(float(&p, "x")?) as f32, snap(float(&p, "y")?) as f32))
    };
    let (x, y) = point(first.get("0")?)?;
    let mut path_style = style.path_style();
    path_style.closed = image.get("closed")?;
    let mut builder = Path::builder(path_style, x, y);
    for cmd in rest {
        builder = segment(builder, cmd, &point)?;
    }
    Ok(Some(builder.build()))
}

/// `builder` with the segment of `cmd` appended.
fn segment(
    builder: PathBuilder,
    cmd: &Object,
    point: &impl Fn(Object) -> Result<(f32, f32)>,
) -> Result<PathBuilder> {
    let at = |name: &str| point(cmd.get(name)?);
    Ok(match class(cmd)?.as_str() {
        "MoveTo" => {
            let (x, y) = at("0")?;
            builder.move_to(x, y)
        }
        "LineTo" => {
            let (x, y) = at("0")?;
            builder.line_to(x, y)
        }
        "QuadTo" => {
            let (cx, cy) = at("control")?;
            let (x, y) = at("end")?;
            builder.quad_to(cx, cy, x, y)
        }
        "CubicTo" => {
            let (c1x, c1y) = at("c1")?;
            let (c2x, c2y) = at("c2")?;
            let (x, y) = at("end")?;
            builder.cubic_to(c1x, c1y, c2x, c2y, x, y)
        }
        "ArcTo" => {
            let (x, y) = at("end")?;
            builder.arc_to(
                float(cmd, "rx")? as f32,
                float(cmd, "ry")? as f32,
                float(cmd, "rotation")? as f32,
                cmd.get("large_arc")?,
                cmd.get("sweep")?,
                x,
                y,
            )
        }
        other => return Err(type_error(cmd.ctx(), &format!("cannot draw a {other}"))),
    })
}

/// The text of a `Text`, or `None` when it draws nothing.
fn text(image: &Object) -> Result<Option<Text>> {
    let style = Style::read(&image.get("style")?)?;
    let font: Object = image.get("font")?;
    let spec = text_spec(image.get("text")?, &font)?;
    let Some(mut text) = spec.fit_transform(affine(&image.get("box")?)?) else {
        return Ok(None);
    };
    let path_style = style.path_style();
    text.fill = path_style.fill;
    text.stroke = path_style.stroke;
    text.stroke_width = path_style.stroke_width;
    text.underline = font.get("underline")?;
    Ok(Some(text))
}

/// `system.text_metrics` gives the width and the height of the box around
/// `text` in `font`, a `Font` of `sgleam/font`. sinteract measures, with the
/// fonts that it embeds, so a text has the same size on every target. A size
/// that cannot draw measures nothing.
pub fn text_metrics<'js>(text: String, font: Object<'js>) -> Result<List<(f64, f64)>> {
    let spec = text_spec(text, &font)?;
    let size = match sinteract::text::measure(
        &spec.family,
        spec.weight,
        spec.style,
        spec.size,
        &spec.text,
    ) {
        Some(m) => (f64::from(m.width()), f64::from(m.height())),
        None => (0.0, 0.0),
    };
    Ok(List(size))
}

/// The spec of `text` in `font`, a `Font` of `sgleam/font`.
fn text_spec(text: String, font: &Object) -> Result<TextSpec> {
    Ok(TextSpec {
        size: font.get::<_, f64>("size")? as f32,
        family: font.get::<_, String>("family")?.into(),
        weight: match class(&font.get("font_weight")?)?.as_str() {
            "Light" => 300,
            "Bold" => 700,
            _ => 400,
        },
        style: match class(&font.get("font_style")?)?.as_str() {
            "Italic" => FontStyle::Italic,
            "Slant" => FontStyle::Oblique,
            _ => FontStyle::Normal,
        },
        text,
    })
}

/// The affine of a `Box`, which takes the unit square centred on the origin
/// to the box.
fn affine(b: &Object) -> Result<[f32; 6]> {
    let point = |name: &str| -> Result<(f32, f32)> {
        let p: Object = b.get(name)?;
        Ok((float(&p, "x")? as f32, float(&p, "y")? as f32))
    };
    let (cx, cy) = point("center")?;
    let (ux, uy) = point("x_axis")?;
    let (vx, vy) = point("y_axis")?;
    Ok([ux, uy, vx, vy, cx, cy])
}

/// A `Style` of `sgleam/style`, with a default for each attribute left out.
struct Style {
    fill: Option<Color>,
    fill_opacity: f32,
    fill_rule: FillRule,
    stroke: Option<Color>,
    stroke_width: Option<f32>,
    stroke_opacity: f32,
    dash: Option<Vec<f32>>,
    line_cap: Option<LineCap>,
    line_join: Option<LineJoin>,
}

impl Style {
    /// Reads the attributes of `style`. Of two that set the same thing, the
    /// first wins, as in an svg element with the attribute twice.
    fn read(style: &Object) -> Result<Self> {
        let mut s = Style {
            fill: None,
            fill_opacity: 1.0,
            fill_rule: FillRule::NonZero,
            stroke: None,
            stroke_width: None,
            stroke_opacity: 1.0,
            dash: None,
            line_cap: None,
            line_join: None,
        };
        let mut seen = Vec::new();
        for attr in list::<Object>(style.get("attributes")?)? {
            let class = class(&attr)?;
            if seen.contains(&class) {
                continue;
            }
            match class.as_str() {
                "Fill" => s.fill = Some(color(&attr.get("0")?)?),
                "FillOpacity" => s.fill_opacity = attr.get::<_, f64>("0")? as f32,
                "FillRule" => {
                    if attr.get::<_, String>("0")? == "evenodd" {
                        s.fill_rule = FillRule::EvenOdd;
                    }
                }
                "Stroke" => s.stroke = Some(color(&attr.get("0")?)?),
                "StrokeWidth" => s.stroke_width = Some(attr.get::<_, f64>("0")? as f32),
                "StrokeOpacity" => s.stroke_opacity = attr.get::<_, f64>("0")? as f32,
                "StrokeDashArray" => {
                    let values: Vec<Value> = list(attr.get("0")?)?;
                    s.dash = Some(values.iter().map(|v| number(v) as f32).collect());
                }
                "StrokeLineCap" => {
                    s.line_cap = Some(match attr.get::<_, String>("0")?.as_str() {
                        "round" => LineCap::Round,
                        "square" => LineCap::Square,
                        _ => LineCap::Butt,
                    });
                }
                "StrokeLineJoin" => {
                    s.line_join = Some(match attr.get::<_, String>("0")?.as_str() {
                        "round" => LineJoin::Round,
                        "bevel" => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    });
                }
                other => {
                    return Err(type_error(attr.ctx(), &format!("unknown style {other}")));
                }
            }
            seen.push(class);
        }
        Ok(s)
    }

    /// Returns `true` if the path draws a thin outline on half pixels, `false`
    /// otherwise.
    fn aligned(&self) -> bool {
        self.stroke.is_some() && self.fill.is_none() && self.stroke_width.is_none()
    }

    fn path_style(&self) -> PathStyle {
        let outline = self.stroke.is_some();
        // A shape with no fill is black, as in svg, and an outline is not
        // filled.
        let fill = match self.fill {
            Some(c) => c,
            None if outline => Color::default(),
            None => Color {
                r: 0,
                g: 0,
                b: 0,
                a: 1.0,
            },
        };
        let stroke = self.stroke.unwrap_or_default();
        PathStyle {
            fill: paint(fill, self.fill_opacity),
            stroke: paint(stroke, self.stroke_opacity),
            stroke_width: if outline {
                self.stroke_width.unwrap_or(1.0)
            } else {
                0.0
            },
            line_cap: self.line_cap.unwrap_or(if outline {
                LineCap::Round
            } else {
                LineCap::Butt
            }),
            line_join: self.line_join.unwrap_or(LineJoin::Miter),
            fill_rule: self.fill_rule,
            // An outline with no join of its own gets a limit of 10.
            miter_limit: if self.line_join.is_none() && outline {
                10.0
            } else {
                sinteract::scene::DEFAULT_MITER_LIMIT
            },
            dash: self.dash.as_ref().and_then(|a| Dash::new(a.clone(), 0.0)),
            closed: false,
        }
    }
}

/// A `Color` of `sgleam/color`, with the alpha as a fraction from 0 to 1.
#[derive(Clone, Copy, Default)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: f32,
}

/// The color with its alpha times `opacity`, rounded once to the byte of the
/// scene.
fn paint(c: Color, opacity: f32) -> Paint {
    let a = (c.a * opacity).clamp(0.0, 1.0) * 255.0;
    Paint::rgba(c.r, c.g, c.b, a.round() as u8)
}

/// A `Color` of `sgleam/color`. `None` has no alpha.
fn color(c: &Object) -> Result<Color> {
    if class(c)? == "None" {
        return Ok(Color::default());
    }
    let channel = |name: &str| -> Result<u8> { Ok(number(&c.get(name)?).clamp(0.0, 255.0) as u8) };
    Ok(Color {
        r: channel("0")?,
        g: channel("1")?,
        b: channel("2")?,
        a: c.get::<_, f64>("3")? as f32,
    })
}

/// The name of the constructor of `value`, which is the name of its variant.
fn class(value: &Object) -> Result<String> {
    value.get::<_, Object>("constructor")?.get("name")
}

/// The elements of a Gleam list.
fn list<'js, T: FromJs<'js>>(mut list: Object<'js>) -> Result<Vec<T>> {
    let mut items = Vec::new();
    while class(&list)? == "NonEmpty" {
        items.push(list.get("head")?);
        list = list.get("tail")?;
    }
    Ok(items)
}

/// The number at `obj.name`, or 0 when it is within [`FP_EPSILON`] of 0. A
/// sine or a cosine leaves a residue near 0 that differs from one libm to
/// another, and the svg would write the residue in full.
fn float(obj: &Object, name: &str) -> Result<f64> {
    let v: f64 = obj.get(name)?;
    Ok(if v.abs() < FP_EPSILON { 0.0 } else { v })
}

/// A number, which an `Int` is too, as a `BigInt` when the program runs with
/// big integers. Anything else is 0.
fn number(v: &Value) -> f64 {
    if let Some(n) = v.as_number() {
        n
    } else if let Some(n) = v.as_big_int() {
        n.clone().to_i64().map_or(0.0, |n| n as f64)
    } else {
        0.0
    }
}

fn type_error(ctx: &Ctx, message: &str) -> rquickjs::Error {
    Exception::throw_type(ctx, message)
}
