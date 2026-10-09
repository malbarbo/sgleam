//! The svg of an `Image` of `sgleam/image`. The image is a value of the
//! compiled JavaScript, and this module reads it into a sinteract [`Scene`],
//! which the svg renderer of sinteract writes out.

use std::sync::atomic::{AtomicUsize, Ordering};

use rquickjs::{Ctx, Exception, FromJs, Object, Result, Value};
use sinteract::asset::decode_data_uri;
use sinteract::renderer::Renderer;
use sinteract::renderer::svg::SvgRenderer;
use sinteract::scene::{
    Bitmap, ClipPath, Dash, FillRule, FontStyle, Image, LineCap, LineJoin, Paint, Path,
    PathBuilder, PathStyle, Sampling, Scene, TextSpec,
};

/// The number in the id prefix of the next svg. The page puts every svg of a
/// run into one HTML document, so two svgs must not share an id.
static NEXT_SVG: AtomicUsize = AtomicUsize::new(0);

/// The svg of `image` on a canvas of `width` by `height`.
pub fn to_svg<'js>(image: Object<'js>, width: f64, height: f64) -> Result<String> {
    let mut scene = Scene::new(width as f32, height as f32);
    draw(&image, &mut scene)?;
    let prefix = format!("s{}-", NEXT_SVG.fetch_add(1, Ordering::Relaxed));
    let mut renderer = SvgRenderer::with_id_prefix(&prefix).expect("a valid id prefix");
    let Ok(svg) = renderer.render(&scene);
    Ok(svg.to_owned())
}

/// Puts `image` into `scene`.
fn draw(image: &Object, scene: &mut Scene) -> Result<()> {
    match class(image)?.as_str() {
        "Path" => {
            if let Some(path) = path(image)? {
                scene.add_path(path);
            }
        }
        "Combination" => {
            draw(&image.get("0")?, scene)?;
            draw(&image.get("1")?, scene)?;
        }
        "Crop" => {
            let clip = ClipPath::from_transform(affine(&image.get("box")?)?);
            let inside: Object = image.get("image")?;
            scene.clip(clip, |clip| draw(&inside, clip))?;
        }
        "Text" => text(image, scene)?,
        "Bitmap" => {
            let transform = affine(&image.get("box")?)?;
            let data_uri: String = image.get("data_uri")?;
            // A bitmap that did not load has an empty data URI and no size.
            if let Some(image) = decode_data_uri(&data_uri).and_then(|b| Image::load(b).ok()) {
                scene.add_bitmap(Bitmap {
                    image,
                    transform,
                    sampling: Sampling::Smooth,
                });
            }
        }
        other => return Err(type_error(image.ctx(), &format!("cannot draw a {other}"))),
    }
    Ok(())
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
            (v + 1.0e-10).floor() + 0.5
        } else {
            v
        }
    };
    let point = |p: Object| -> Result<(f32, f32)> {
        Ok((snap(p.get("x")?) as f32, snap(p.get("y")?) as f32))
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
                cmd.get::<_, f64>("rx")? as f32,
                cmd.get::<_, f64>("ry")? as f32,
                cmd.get::<_, f64>("rotation")? as f32,
                cmd.get("large_arc")?,
                cmd.get("sweep")?,
                x,
                y,
            )
        }
        other => return Err(type_error(cmd.ctx(), &format!("cannot draw a {other}"))),
    })
}

/// Puts the text of a `Text` into `scene`, unless it draws nothing.
fn text(image: &Object, scene: &mut Scene) -> Result<()> {
    let style = Style::read(&image.get("style")?)?;
    let font: Object = image.get("font")?;
    let spec = TextSpec {
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
        text: image.get("text")?,
    };
    let Some(mut text) = spec.fit_transform(affine(&image.get("box")?)?) else {
        return Ok(());
    };
    let path_style = style.path_style();
    text.fill = path_style.fill;
    text.stroke = path_style.stroke;
    text.stroke_width = path_style.stroke_width;
    text.underline = font.get("underline")?;
    scene.add_text(text);
    Ok(())
}

/// The affine of a `Box`, which takes the unit square centred on the origin
/// to the box.
fn affine(b: &Object) -> Result<[f32; 6]> {
    let point = |name: &str| -> Result<(f32, f32)> {
        let p: Object = b.get(name)?;
        Ok((p.get::<_, f64>("x")? as f32, p.get::<_, f64>("y")? as f32))
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
