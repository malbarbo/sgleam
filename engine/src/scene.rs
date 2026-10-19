//! The svg of an `Image` of `sgleam/image`. The image is a value of the
//! compiled JavaScript, and this module reads it into a sinteract [`Scene`],
//! which the svg renderer of sinteract writes out.
//!
//! The size of a text comes from here too, so the text measures with the
//! face that draws it, and so does the box of a path.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use kurbo::{BezPath, ParamCurveExtrema, Point, Rect, SvgArc, Vec2};
use rquickjs::class::{JsClass, Readable, Trace, Tracer};
use rquickjs::function::This;
use rquickjs::{
    Class, Constructor, Ctx, Exception, FromJs, Function, JsLifetime, Object, Result, Value,
    convert::List,
};
use sinteract::asset::{ImageHead, decode_data_uri};
use sinteract::renderer::Renderer;
use sinteract::renderer::pixmap::render_to_pixmap;
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
    let scene = to_scene(&image, width, height)?;
    let prefix = format!("s{}-", NEXT_SVG.fetch_add(1, Ordering::Relaxed));
    let mut renderer = SvgRenderer::with_id_prefix(&prefix)
        .expect("a valid id prefix")
        .keep_image_formats();
    let Ok(svg) = renderer.render(&scene);
    Ok(svg.to_owned())
}

/// Prints `image` in the terminal on a canvas of `width` by `height`.
/// Returns `true` if the image went out, `false` otherwise, and then the repl
/// prints the word `Image`.
#[cfg(not(target_arch = "wasm32"))]
pub fn print_image<'js>(image: Object<'js>, width: f64, height: f64) -> Result<bool> {
    Ok(crate::host::print_scene(&to_scene(&image, width, height)?))
}

/// Shows `image` as a frame of a world, on a canvas of `width` by `height`.
/// Returns `true` if the display of the world is open, `false` otherwise, and
/// then the world prints the word `Image`.
pub fn show_image<'js>(image: Object<'js>, width: f64, height: f64) -> Result<bool> {
    Ok(crate::host::show_scene(to_scene(&image, width, height)?))
}

/// Keeps `image` as the next frame of one player of a multiplayer world.
pub fn show_player_image<'js>(
    player: u32,
    image: Object<'js>,
    width: f64,
    height: f64,
) -> Result<bool> {
    Ok(crate::host::show_player_scene(
        player,
        to_scene(&image, width, height)?,
    ))
}

/// `image` drawn on a canvas of `width` by `height`.
fn to_scene(image: &Object, width: f64, height: f64) -> Result<Scene> {
    let mut scene = Scene::new(width as f32, height as f32);
    draw(leaves(image)?, &mut scene);
    Ok(scene)
}

/// `same_pixels` of `sgleam/image` returns `true` if `a` and `b` draw the
/// same pixels on a canvas of `width` by `height`, `false` otherwise. The
/// pixmap draws a marker for an image that is not a PNG, so the bitmaps of
/// such images also have to show the same images in the same order.
pub fn same_pixels<'js>(
    ctx: Ctx<'js>,
    a: Object<'js>,
    b: Object<'js>,
    width: f64,
    height: f64,
) -> Result<bool> {
    let a = leaves(&a)?;
    let b = leaves(&b)?;
    if other_images(&a) != other_images(&b) {
        return Ok(false);
    }
    Ok(pixels(&ctx, a, width, height)? == pixels(&ctx, b, width, height)?)
}

/// The images of the bitmaps in `runs` that are not a PNG.
fn other_images(runs: &[Run]) -> Vec<&Image> {
    runs.iter()
        .flat_map(|(_, leaves)| leaves)
        .filter_map(|leaf| match leaf {
            Leaf::Bitmap { bitmap, png: false } => Some(&bitmap.image),
            _ => None,
        })
        .collect()
}

/// The pixels of `runs` on a canvas of `width` by `height`.
fn pixels(ctx: &Ctx, runs: Vec<Run>, width: f64, height: f64) -> Result<Vec<u8>> {
    let mut scene = Scene::new(width as f32, height as f32);
    draw(runs, &mut scene);
    match render_to_pixmap(&scene, 1.0) {
        Ok(pixmap) => Ok(pixmap.take()),
        Err(_) => Err(Exception::throw_range(
            ctx,
            "the images are too large to compare by their pixels",
        )),
    }
}

/// The elements of `image`, in runs that share a region, in drawing order.
fn leaves(image: &Object) -> Result<Vec<Run>> {
    let mut runs = Vec::new();
    walk(image, (0.0, 0.0), &None, &mut runs)?;
    Ok(runs)
}

/// Puts `runs` into `scene`. A crop inside another becomes one clip of the
/// two boxes together, so no clip of the scene holds another, and the
/// nesting limit of sinteract drops nothing.
fn draw(runs: Vec<Run>, scene: &mut Scene) {
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
}

/// The area that the crops around an element leave visible, as the corners
/// of a convex polygon. `None` is no crop, and no corners is an area with
/// nothing in it.
type Region = Option<Vec<(f64, f64)>>;

/// How far the nodes above an element move it, as the offsets of the
/// children of `Combination` and `Crop` say.
type Offset = (f64, f64);

/// Elements in a row that share a region.
type Run = (Region, Vec<Leaf>);

/// An element of the scene, before it goes into a clip.
enum Leaf {
    Path(Path),
    Text(Text),
    Bitmap { bitmap: Bitmap, png: bool },
}

impl Leaf {
    fn add_to(self, scene: &mut Scene) {
        match self {
            Leaf::Path(path) => scene.add_path(path),
            Leaf::Text(text) => scene.add_text(text),
            Leaf::Bitmap { bitmap, .. } => scene.add_bitmap(bitmap),
        }
    }
}

/// Appends the elements of `image`, moved by `at`, to `runs`, which groups
/// the elements in a row that share a region, in drawing order. The nodes
/// wait on a stack of their own, so a deep image does not overflow the stack
/// of the thread.
fn walk<'js>(image: &Object<'js>, at: Offset, region: &Region, runs: &mut Vec<Run>) -> Result<()> {
    let mut pending = vec![(image.clone(), at, Rc::new(region.clone()))];
    while let Some((image, at, region)) = pending.pop() {
        let leaf = match class(&image)?.as_str() {
            "Path" => path(&image, at)?.map(Leaf::Path),
            "Line" => Some(Leaf::Path(line(&image, at)?)),
            "Combination" => {
                // The stack gives back the last node first, and `a` draws
                // before `b`.
                let b_at = moved(at, image.get("b_at")?)?;
                pending.push((image.get("b")?, b_at, region.clone()));
                let a_at = moved(at, image.get("a_at")?)?;
                pending.push((image.get("a")?, a_at, region.clone()));
                None
            }
            "Crop" => {
                let corners = corners(affine(&image.get("box")?, at)?);
                let inside = intersect(&region, &corners);
                let inner = moved(at, image.get("at")?)?;
                pending.push((image.get("image")?, inner, Rc::new(inside)));
                None
            }
            "Text" => text(&image, at)?.map(Leaf::Text),
            "Bitmap" => {
                let transform = affine(&image.get("box")?, at)?;
                let loaded: MaybeLoaded = image.get("image")?;
                loaded.map(|loaded| {
                    let loaded = loaded.borrow();
                    Leaf::Bitmap {
                        bitmap: Bitmap {
                            image: loaded.image.clone(),
                            transform,
                            sampling: Sampling::Smooth,
                        },
                        png: loaded.png,
                    }
                })
            }
            other => return Err(type_error(image.ctx(), &format!("cannot draw a {other}"))),
        };
        let Some(leaf) = leaf else {
            continue;
        };
        if region.as_ref().as_ref().is_some_and(Vec::is_empty) {
            continue;
        }
        match runs.last_mut() {
            Some((last, leaves)) if last == region.as_ref() => leaves.push(leaf),
            _ => runs.push((region.as_ref().clone(), vec![leaf])),
        }
    }
    Ok(())
}

/// `at` moved by the point `by`.
fn moved(at: Offset, by: Object) -> Result<Offset> {
    Ok((at.0 + float(&by, "x")?, at.1 + float(&by, "y")?))
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
pub struct Loaded {
    image: Image,
    /// Whether the image is a PNG, the one format that the pixmap decodes.
    png: bool,
}

impl Loaded {
    fn new(image: Image, head: ImageHead) -> Self {
        let png = head.mime == "image/png";
        Self { image, png }
    }
}

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
                .is_some_and(|other| this.borrow().image == other.borrow().image)
        };
        proto.set("equals", Function::new(ctx.clone(), equals)?)?;
        Ok(Some(proto))
    }

    fn constructor(_ctx: &Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        Ok(None)
    }
}

/// The image of a bitmap, or `None` for the one of `system.no_image`.
type MaybeLoaded<'js> = Option<Class<'js, Loaded>>;

/// The width and the height of an image, and the image.
type Sized<'js> = List<(f64, f64, Class<'js, Loaded>)>;

/// `system.load_image` gives the width and the height of the image in
/// `data_uri`, and its image, or nothing when the data URI holds no image. An
/// image larger than the limit of sinteract shrinks, and the size stays the
/// one of the original.
pub fn load_image<'js>(ctx: Ctx<'js>, data_uri: String) -> Result<Option<Sized<'js>>> {
    let Some((image, head)) =
        decode_data_uri(&data_uri).and_then(|blob| Image::load_with_head(blob).ok())
    else {
        return Ok(None);
    };
    if head.width == 0 || head.height == 0 {
        return Ok(None);
    }
    Ok(Some(List((
        f64::from(head.width),
        f64::from(head.height),
        Class::instance(ctx, Loaded::new(image, head))?,
    ))))
}

/// The path of a `Path`, or `None` when its commands do not start with a
/// move, which svg does not draw either.
fn path(image: &Object, at: Offset) -> Result<Option<Path>> {
    let style = Style::read(&image.get("style")?)?;
    let commands: Vec<Object> = list(image.get("commands")?)?;
    let Some((first, rest)) = commands.split_first() else {
        return Ok(None);
    };
    if class(first)? != "MoveTo" {
        return Ok(None);
    }
    let point = pixel_point(&style, at);
    let (x, y) = point(first.get("0")?)?;
    let mut path_style = style.path_style();
    path_style.closed = image.get("closed")?;
    let mut builder = Path::builder(path_style, x, y);
    for cmd in rest {
        builder = segment(builder, cmd, &point)?;
    }
    Ok(Some(builder.build()))
}

/// The path of a `Line`, from one end to the other.
fn line(image: &Object, at: Offset) -> Result<Path> {
    let style = Style::read(&image.get("style")?)?;
    let point = pixel_point(&style, at);
    let (x, y) = point(image.get("from")?)?;
    let (to_x, to_y) = point(image.get("to")?)?;
    Ok(Path::builder(style.path_style(), x, y)
        .line_to(to_x, to_y)
        .build())
}

/// Reads a point for a path with `style`, moved by `at`. A thin outline on
/// a half pixel covers one row of pixels, and on a whole pixel it blurs over
/// two, so an aligned style puts the point on the center of its pixel. The
/// epsilon takes 9.999999999999998 as 10.
fn pixel_point(style: &Style, at: Offset) -> impl Fn(Object) -> Result<(f32, f32)> + '_ {
    let snap = |v: f64| {
        if style.aligned() {
            (v + FP_EPSILON).floor() + 0.5
        } else {
            v
        }
    };
    move |p: Object| {
        Ok((
            snap(float(&p, "x")? + at.0) as f32,
            snap(float(&p, "y")? + at.1) as f32,
        ))
    }
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

/// `path_box` of `sgleam/image` gives the box around the path of
/// `commands` as the least x, the least y, the greatest x and the greatest y.
/// An arc counts as the cubics that sinteract draws for it.
pub fn path_box<'js>(commands: Object<'js>) -> Result<List<(f64, f64, f64, f64)>> {
    let mut path = BezPath::new();
    for cmd in list::<Object>(commands)? {
        let at = |name: &str| -> Result<Point> {
            let p: Object = cmd.get(name)?;
            Ok(Point::new(p.get("x")?, p.get("y")?))
        };
        let kind = class(&cmd)?;
        let end = at(match kind.as_str() {
            "MoveTo" | "LineTo" => "0",
            _ => "end",
        })?;
        // A path that starts with a segment other than a move starts at the
        // end of that segment.
        let Some(from) = path.elements().last().and_then(|el| el.end_point()) else {
            path.move_to(end);
            continue;
        };
        match kind.as_str() {
            "MoveTo" => path.move_to(end),
            "LineTo" => path.line_to(end),
            "QuadTo" => path.quad_to(at("control")?, end),
            "CubicTo" => path.curve_to(at("c1")?, at("c2")?, end),
            "ArcTo" => {
                let arc = SvgArc {
                    from,
                    to: end,
                    radii: Vec2::new(cmd.get("rx")?, cmd.get("ry")?),
                    x_rotation: cmd.get::<_, f64>("rotation")?.to_radians(),
                    large_arc: cmd.get("large_arc")?,
                    sweep: cmd.get("sweep")?,
                };
                match kurbo::Arc::from_svg_arc(&arc) {
                    Some(arc) => {
                        let tolerance = arc_tolerance(arc.radii.x.max(arc.radii.y));
                        path.extend(arc.append_iter(tolerance));
                    }
                    None => path.line_to(end),
                }
            }
            other => {
                return Err(type_error(cmd.ctx(), &format!("cannot measure a {other}")));
            }
        }
    }
    // A move adds its point even when no segment leaves it.
    let mut points = path.elements().iter().filter_map(|el| el.end_point());
    let Some(first) = points.next() else {
        return Ok(List((0.0, 0.0, 0.0, 0.0)));
    };
    let rect = points.fold(Rect::from_points(first, first), |rect, p| rect.union_pt(p));
    let rect = path
        .segments()
        .fold(rect, |rect, seg| rect.union(seg.bounding_box()));
    Ok(List((rect.x0, rect.y0, rect.x1, rect.y1)))
}

/// The tolerance that sinteract takes to turn an arc of `radius` into
/// cubics. It grows with the radius, so one turn takes at most 64 cubics.
fn arc_tolerance(radius: f64) -> f64 {
    0.1f64.max(1.1163 * radius / 64f64.powi(6))
}

/// The text of a `Text`, or `None` when it draws nothing.
fn text(image: &Object, at: Offset) -> Result<Option<Text>> {
    let style = Style::read(&image.get("style")?)?;
    let font: Object = image.get("font")?;
    let spec = text_spec(image.get("text")?, &font)?;
    let Some(mut text) = spec.fit_transform(affine(&image.get("box")?, at)?) else {
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
/// to the box moved by `at`.
fn affine(b: &Object, at: Offset) -> Result<[f32; 6]> {
    let point = |name: &str| -> Result<(f32, f32)> {
        let p: Object = b.get(name)?;
        Ok((float(&p, "x")? as f32, float(&p, "y")? as f32))
    };
    let (cx, cy) = moved(at, b.get("center")?)?;
    let (cx, cy) = (cx as f32, cy as f32);
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
    /// otherwise. A width of 1 is the width an outline has by default, so it
    /// draws the same.
    fn aligned(&self) -> bool {
        self.stroke.is_some() && self.fill.is_none() && self.stroke_width.is_none_or(|w| w == 1.0)
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
