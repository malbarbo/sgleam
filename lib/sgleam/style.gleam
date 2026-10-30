import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string
import sgleam/color.{type Color}

/// A field is `None` for an attribute that the style leaves to the shape.
pub opaque type Style {
  Style(
    fill: Option(Color),
    fill_opacity: Option(Float),
    fill_rule: Option(String),
    stroke: Option(Color),
    stroke_linecap: Option(String),
    stroke_linejoin: Option(String),
    stroke_width: Option(Float),
    stroke_opacity: Option(Float),
    stroke_dash_array: Option(List(Float)),
  )
}

/// The attributes of every style in `styles`. Of two styles that set the
/// same attribute, the first wins.
pub fn join(styles: List(Style)) -> Style {
  list.fold(styles, none, fn(joined, style) {
    Style(
      fill: option.or(joined.fill, style.fill),
      fill_opacity: option.or(joined.fill_opacity, style.fill_opacity),
      fill_rule: option.or(joined.fill_rule, style.fill_rule),
      stroke: option.or(joined.stroke, style.stroke),
      stroke_linecap: option.or(joined.stroke_linecap, style.stroke_linecap),
      stroke_linejoin: option.or(joined.stroke_linejoin, style.stroke_linejoin),
      stroke_width: option.or(joined.stroke_width, style.stroke_width),
      stroke_opacity: option.or(joined.stroke_opacity, style.stroke_opacity),
      stroke_dash_array: option.or(
        joined.stroke_dash_array,
        style.stroke_dash_array,
      ),
    )
  })
}

pub const none = Style(
  fill: None,
  fill_opacity: None,
  fill_rule: None,
  stroke: None,
  stroke_linecap: None,
  stroke_linejoin: None,
  stroke_width: None,
  stroke_opacity: None,
  stroke_dash_array: None,
)

/// The text `inspect` prints for `style`, an expression the user could write.
@internal
pub fn to_source(style: Style) -> String {
  let sources =
    option.values([
      option.map(style.fill, fn(c) { "fill." <> color.name(c) }),
      option.map(style.fill_opacity, fn(v) {
        "fill." <> number_call("opacity", v)
      }),
      option.map(style.fill_rule, fn(rule) { "fill.rule_" <> rule }),
      option.map(style.stroke, fn(c) { "stroke." <> color.name(c) }),
      option.map(style.stroke_linecap, fn(cap) { "stroke.linecap_" <> cap }),
      option.map(style.stroke_linejoin, fn(join) { "stroke.linejoin_" <> join }),
      option.map(style.stroke_width, fn(v) {
        "stroke." <> number_call("width", v)
      }),
      option.map(style.stroke_opacity, fn(v) {
        "stroke." <> number_call("opacity", v)
      }),
      option.map(style.stroke_dash_array, dash_array_source),
    ])
  case sources {
    [] -> "style.none"
    [one] -> one
    many -> "style.join([" <> string.join(many, ", ") <> "])"
  }
}

const named_dash_arrays = [
  #(stroke_dashed, "stroke.dashed"),
  #(stroke_dotted, "stroke.dotted"),
  #(stroke_long_dashed, "stroke.long_dashed"),
  #(stroke_dash_dot, "stroke.dash_dot"),
]

fn dash_array_source(values: List(Float)) -> String {
  let style = Style(..none, stroke_dash_array: Some(values))
  case list.key_find(named_dash_arrays, style) {
    Ok(name) -> name
    Error(_) -> "stroke.dash_array([" <> numbers(values) <> "])"
  }
}

/// `name(v)` for a whole `v`, the function that takes an `Int`, and
/// `namef(v)` otherwise.
fn number_call(name: String, v: Float) -> String {
  case whole(v) {
    Ok(n) -> name <> "(" <> int.to_string(n) <> ")"
    Error(_) -> name <> "f(" <> float.to_string(v) <> ")"
  }
}

/// The values of a dash array, as `Int` when all are whole. Only `scale`
/// makes them fractional, and `stroke.dash_array` has no `Float` version.
fn numbers(values: List(Float)) -> String {
  case list.try_map(values, whole) {
    Ok(ns) -> ns |> list.map(int.to_string) |> string.join(", ")
    Error(_) -> values |> list.map(float.to_string) |> string.join(", ")
  }
}

fn whole(v: Float) -> Result(Int, Nil) {
  let n = float.round(v)
  case int.to_float(n) == v {
    True -> Ok(n)
    False -> Error(Nil)
  }
}

/// `style` for a shape scaled by `factor`. A stroke with no width of its
/// own stays one pixel wide, as an outline does in 2htdp.
pub fn scale(style: Style, factor: Float) -> Style {
  Style(
    ..style,
    stroke_width: option.map(style.stroke_width, fn(v) { v *. factor }),
    stroke_dash_array: option.map(style.stroke_dash_array, fn(values) {
      list.map(values, fn(v) { v *. factor })
    }),
  )
}

pub fn fill(c: Color) -> Style {
  Style(..none, fill: Some(c))
}

pub fn fill_opacity(opacity: Int) -> Style {
  fill_opacityf(int.to_float(opacity))
}

pub fn fill_opacityf(opacity: Float) -> Style {
  Style(..none, fill_opacity: Some(float.clamp(opacity, 0.0, 1.0)))
}

pub fn stroke(c: Color) -> Style {
  Style(..none, stroke: Some(c))
}

pub fn stroke_width(width: Int) -> Style {
  stroke_widthf(int.to_float(width))
}

pub fn stroke_widthf(width: Float) -> Style {
  Style(..none, stroke_width: Some(positive(width)))
}

pub fn stroke_opacity(opacity: Int) -> Style {
  stroke_opacityf(int.to_float(opacity))
}

pub fn stroke_opacityf(opacity: Float) -> Style {
  Style(..none, stroke_opacity: Some(float.clamp(opacity, 0.0, 1.0)))
}

pub fn stroke_dash_array(values: List(Int)) -> Style {
  let values = list.map(values, fn(v) { int.to_float(int.max(0, v)) })
  Style(..none, stroke_dash_array: Some(values))
}

pub const stroke_dashed = Style(..none, stroke_dash_array: Some([6.0, 3.0]))

pub const stroke_dotted = Style(..none, stroke_dash_array: Some([2.0, 2.0]))

pub const stroke_long_dashed = Style(
  ..none,
  stroke_dash_array: Some([12.0, 4.0]),
)

pub const stroke_dash_dot = Style(
  ..none,
  stroke_dash_array: Some([6.0, 3.0, 2.0, 3.0]),
)

fn positive(n: Float) -> Float {
  float.max(0.0, n)
}

// **************************
// * Fill
// **************************

pub const fill_rule_nonzero = Style(..none, fill_rule: Some("nonzero"))

pub const fill_rule_evenodd = Style(..none, fill_rule: Some("evenodd"))

pub fn fill_rgb(red: Int, green: Int, blue: Int) -> Style {
  Style(..none, fill: Some(color.rgb(red, green, blue)))
}

pub fn fill_rgba(red: Int, green: Int, blue: Int, alpha: Float) -> Style {
  Style(..none, fill: Some(color.rgba(red, green, blue, alpha)))
}

pub const fill_none = Style(..none, fill: Some(color.none))

pub const fill_aliceblue = Style(..none, fill: Some(color.aliceblue))

pub const fill_antiquewhite = Style(..none, fill: Some(color.antiquewhite))

pub const fill_aqua = Style(..none, fill: Some(color.aqua))

pub const fill_aquamarine = Style(..none, fill: Some(color.aquamarine))

pub const fill_azure = Style(..none, fill: Some(color.azure))

pub const fill_beige = Style(..none, fill: Some(color.beige))

pub const fill_bisque = Style(..none, fill: Some(color.bisque))

pub const fill_black = Style(..none, fill: Some(color.black))

pub const fill_blanchedalmond = Style(..none, fill: Some(color.blanchedalmond))

pub const fill_blue = Style(..none, fill: Some(color.blue))

pub const fill_blueviolet = Style(..none, fill: Some(color.blueviolet))

pub const fill_brown = Style(..none, fill: Some(color.brown))

pub const fill_burlywood = Style(..none, fill: Some(color.burlywood))

pub const fill_cadetblue = Style(..none, fill: Some(color.cadetblue))

pub const fill_chartreuse = Style(..none, fill: Some(color.chartreuse))

pub const fill_chocolate = Style(..none, fill: Some(color.chocolate))

pub const fill_coral = Style(..none, fill: Some(color.coral))

pub const fill_cornflowerblue = Style(..none, fill: Some(color.cornflowerblue))

pub const fill_cornsilk = Style(..none, fill: Some(color.cornsilk))

pub const fill_crimson = Style(..none, fill: Some(color.crimson))

pub const fill_cyan = Style(..none, fill: Some(color.cyan))

pub const fill_darkblue = Style(..none, fill: Some(color.darkblue))

pub const fill_darkcyan = Style(..none, fill: Some(color.darkcyan))

pub const fill_darkgoldenrod = Style(..none, fill: Some(color.darkgoldenrod))

pub const fill_darkgray = Style(..none, fill: Some(color.darkgray))

pub const fill_darkgreen = Style(..none, fill: Some(color.darkgreen))

pub const fill_darkgrey = Style(..none, fill: Some(color.darkgrey))

pub const fill_darkkhaki = Style(..none, fill: Some(color.darkkhaki))

pub const fill_darkmagenta = Style(..none, fill: Some(color.darkmagenta))

pub const fill_darkolivegreen = Style(..none, fill: Some(color.darkolivegreen))

pub const fill_darkorange = Style(..none, fill: Some(color.darkorange))

pub const fill_darkorchid = Style(..none, fill: Some(color.darkorchid))

pub const fill_darkred = Style(..none, fill: Some(color.darkred))

pub const fill_darksalmon = Style(..none, fill: Some(color.darksalmon))

pub const fill_darkseagreen = Style(..none, fill: Some(color.darkseagreen))

pub const fill_darkslateblue = Style(..none, fill: Some(color.darkslateblue))

pub const fill_darkslategray = Style(..none, fill: Some(color.darkslategray))

pub const fill_darkslategrey = Style(..none, fill: Some(color.darkslategrey))

pub const fill_darkturquoise = Style(..none, fill: Some(color.darkturquoise))

pub const fill_darkviolet = Style(..none, fill: Some(color.darkviolet))

pub const fill_deeppink = Style(..none, fill: Some(color.deeppink))

pub const fill_deepskyblue = Style(..none, fill: Some(color.deepskyblue))

pub const fill_dimgray = Style(..none, fill: Some(color.dimgray))

pub const fill_dimgrey = Style(..none, fill: Some(color.dimgrey))

pub const fill_dodgerblue = Style(..none, fill: Some(color.dodgerblue))

pub const fill_firebrick = Style(..none, fill: Some(color.firebrick))

pub const fill_floralwhite = Style(..none, fill: Some(color.floralwhite))

pub const fill_forestgreen = Style(..none, fill: Some(color.forestgreen))

pub const fill_fuchsia = Style(..none, fill: Some(color.fuchsia))

pub const fill_gainsboro = Style(..none, fill: Some(color.gainsboro))

pub const fill_ghostwhite = Style(..none, fill: Some(color.ghostwhite))

pub const fill_gold = Style(..none, fill: Some(color.gold))

pub const fill_goldenrod = Style(..none, fill: Some(color.goldenrod))

pub const fill_gray = Style(..none, fill: Some(color.gray))

pub const fill_green = Style(..none, fill: Some(color.green))

pub const fill_greenyellow = Style(..none, fill: Some(color.greenyellow))

pub const fill_grey = Style(..none, fill: Some(color.grey))

pub const fill_honeydew = Style(..none, fill: Some(color.honeydew))

pub const fill_hotpink = Style(..none, fill: Some(color.hotpink))

pub const fill_indianred = Style(..none, fill: Some(color.indianred))

pub const fill_indigo = Style(..none, fill: Some(color.indigo))

pub const fill_ivory = Style(..none, fill: Some(color.ivory))

pub const fill_khaki = Style(..none, fill: Some(color.khaki))

pub const fill_lavender = Style(..none, fill: Some(color.lavender))

pub const fill_lavenderblush = Style(..none, fill: Some(color.lavenderblush))

pub const fill_lawngreen = Style(..none, fill: Some(color.lawngreen))

pub const fill_lemonchiffon = Style(..none, fill: Some(color.lemonchiffon))

pub const fill_lightblue = Style(..none, fill: Some(color.lightblue))

pub const fill_lightcoral = Style(..none, fill: Some(color.lightcoral))

pub const fill_lightcyan = Style(..none, fill: Some(color.lightcyan))

pub const fill_lightgoldenrodyellow = Style(
  ..none,
  fill: Some(color.lightgoldenrodyellow),
)

pub const fill_lightgray = Style(..none, fill: Some(color.lightgray))

pub const fill_lightgreen = Style(..none, fill: Some(color.lightgreen))

pub const fill_lightgrey = Style(..none, fill: Some(color.lightgrey))

pub const fill_lightpink = Style(..none, fill: Some(color.lightpink))

pub const fill_lightsalmon = Style(..none, fill: Some(color.lightsalmon))

pub const fill_lightseagreen = Style(..none, fill: Some(color.lightseagreen))

pub const fill_lightskyblue = Style(..none, fill: Some(color.lightskyblue))

pub const fill_lightslategray = Style(..none, fill: Some(color.lightslategray))

pub const fill_lightslategrey = Style(..none, fill: Some(color.lightslategrey))

pub const fill_lightsteelblue = Style(..none, fill: Some(color.lightsteelblue))

pub const fill_lightyellow = Style(..none, fill: Some(color.lightyellow))

pub const fill_lime = Style(..none, fill: Some(color.lime))

pub const fill_limegreen = Style(..none, fill: Some(color.limegreen))

pub const fill_linen = Style(..none, fill: Some(color.linen))

pub const fill_magenta = Style(..none, fill: Some(color.magenta))

pub const fill_maroon = Style(..none, fill: Some(color.maroon))

pub const fill_mediumaquamarine = Style(
  ..none,
  fill: Some(color.mediumaquamarine),
)

pub const fill_mediumblue = Style(..none, fill: Some(color.mediumblue))

pub const fill_mediumorchid = Style(..none, fill: Some(color.mediumorchid))

pub const fill_mediumpurple = Style(..none, fill: Some(color.mediumpurple))

pub const fill_mediumseagreen = Style(..none, fill: Some(color.mediumseagreen))

pub const fill_mediumslateblue = Style(..none, fill: Some(color.mediumslateblue))

pub const fill_mediumspringgreen = Style(
  ..none,
  fill: Some(color.mediumspringgreen),
)

pub const fill_mediumturquoise = Style(..none, fill: Some(color.mediumturquoise))

pub const fill_mediumvioletred = Style(..none, fill: Some(color.mediumvioletred))

pub const fill_midnightblue = Style(..none, fill: Some(color.midnightblue))

pub const fill_mintcream = Style(..none, fill: Some(color.mintcream))

pub const fill_mistyrose = Style(..none, fill: Some(color.mistyrose))

pub const fill_moccasin = Style(..none, fill: Some(color.moccasin))

pub const fill_navajowhite = Style(..none, fill: Some(color.navajowhite))

pub const fill_navy = Style(..none, fill: Some(color.navy))

pub const fill_oldlace = Style(..none, fill: Some(color.oldlace))

pub const fill_olive = Style(..none, fill: Some(color.olive))

pub const fill_olivedrab = Style(..none, fill: Some(color.olivedrab))

pub const fill_orange = Style(..none, fill: Some(color.orange))

pub const fill_orangered = Style(..none, fill: Some(color.orangered))

pub const fill_orchid = Style(..none, fill: Some(color.orchid))

pub const fill_palegoldenrod = Style(..none, fill: Some(color.palegoldenrod))

pub const fill_palegreen = Style(..none, fill: Some(color.palegreen))

pub const fill_paleturquoise = Style(..none, fill: Some(color.paleturquoise))

pub const fill_palevioletred = Style(..none, fill: Some(color.palevioletred))

pub const fill_papayawhip = Style(..none, fill: Some(color.papayawhip))

pub const fill_peachpuff = Style(..none, fill: Some(color.peachpuff))

pub const fill_peru = Style(..none, fill: Some(color.peru))

pub const fill_pink = Style(..none, fill: Some(color.pink))

pub const fill_plum = Style(..none, fill: Some(color.plum))

pub const fill_powderblue = Style(..none, fill: Some(color.powderblue))

pub const fill_purple = Style(..none, fill: Some(color.purple))

pub const fill_red = Style(..none, fill: Some(color.red))

pub const fill_rosybrown = Style(..none, fill: Some(color.rosybrown))

pub const fill_royalblue = Style(..none, fill: Some(color.royalblue))

pub const fill_saddlebrown = Style(..none, fill: Some(color.saddlebrown))

pub const fill_salmon = Style(..none, fill: Some(color.salmon))

pub const fill_sandybrown = Style(..none, fill: Some(color.sandybrown))

pub const fill_seagreen = Style(..none, fill: Some(color.seagreen))

pub const fill_seashell = Style(..none, fill: Some(color.seashell))

pub const fill_sienna = Style(..none, fill: Some(color.sienna))

pub const fill_silver = Style(..none, fill: Some(color.silver))

pub const fill_skyblue = Style(..none, fill: Some(color.skyblue))

pub const fill_slateblue = Style(..none, fill: Some(color.slateblue))

pub const fill_slategray = Style(..none, fill: Some(color.slategray))

pub const fill_slategrey = Style(..none, fill: Some(color.slategrey))

pub const fill_snow = Style(..none, fill: Some(color.snow))

pub const fill_springgreen = Style(..none, fill: Some(color.springgreen))

pub const fill_steelblue = Style(..none, fill: Some(color.steelblue))

pub const fill_tan = Style(..none, fill: Some(color.tan))

pub const fill_teal = Style(..none, fill: Some(color.teal))

pub const fill_thistle = Style(..none, fill: Some(color.thistle))

pub const fill_tomato = Style(..none, fill: Some(color.tomato))

pub const fill_turquoise = Style(..none, fill: Some(color.turquoise))

pub const fill_violet = Style(..none, fill: Some(color.violet))

pub const fill_wheat = Style(..none, fill: Some(color.wheat))

pub const fill_white = Style(..none, fill: Some(color.white))

pub const fill_whitesmoke = Style(..none, fill: Some(color.whitesmoke))

pub const fill_yellow = Style(..none, fill: Some(color.yellow))

pub const fill_yellowgreen = Style(..none, fill: Some(color.yellowgreen))

// **************************
// * Stroke
// **************************

pub const stroke_linecap_butt = Style(..none, stroke_linecap: Some("butt"))

pub const stroke_linecap_round = Style(..none, stroke_linecap: Some("round"))

pub const stroke_linecap_square = Style(..none, stroke_linecap: Some("square"))

pub const stroke_linejoin_bevel = Style(..none, stroke_linejoin: Some("bevel"))

pub const stroke_linejoin_miter = Style(..none, stroke_linejoin: Some("miter"))

pub const stroke_linejoin_round = Style(..none, stroke_linejoin: Some("round"))

pub fn stroke_rgb(red: Int, green: Int, blue: Int) -> Style {
  Style(..none, stroke: Some(color.rgb(red, green, blue)))
}

pub fn stroke_rgba(red: Int, green: Int, blue: Int, alpha: Float) -> Style {
  Style(..none, stroke: Some(color.rgba(red, green, blue, alpha)))
}

pub const stroke_none = Style(..none, stroke: Some(color.none))

pub const stroke_aliceblue = Style(..none, stroke: Some(color.aliceblue))

pub const stroke_antiquewhite = Style(..none, stroke: Some(color.antiquewhite))

pub const stroke_aqua = Style(..none, stroke: Some(color.aqua))

pub const stroke_aquamarine = Style(..none, stroke: Some(color.aquamarine))

pub const stroke_azure = Style(..none, stroke: Some(color.azure))

pub const stroke_beige = Style(..none, stroke: Some(color.beige))

pub const stroke_bisque = Style(..none, stroke: Some(color.bisque))

pub const stroke_black = Style(..none, stroke: Some(color.black))

pub const stroke_blanchedalmond = Style(
  ..none,
  stroke: Some(color.blanchedalmond),
)

pub const stroke_blue = Style(..none, stroke: Some(color.blue))

pub const stroke_blueviolet = Style(..none, stroke: Some(color.blueviolet))

pub const stroke_brown = Style(..none, stroke: Some(color.brown))

pub const stroke_burlywood = Style(..none, stroke: Some(color.burlywood))

pub const stroke_cadetblue = Style(..none, stroke: Some(color.cadetblue))

pub const stroke_chartreuse = Style(..none, stroke: Some(color.chartreuse))

pub const stroke_chocolate = Style(..none, stroke: Some(color.chocolate))

pub const stroke_coral = Style(..none, stroke: Some(color.coral))

pub const stroke_cornflowerblue = Style(
  ..none,
  stroke: Some(color.cornflowerblue),
)

pub const stroke_cornsilk = Style(..none, stroke: Some(color.cornsilk))

pub const stroke_crimson = Style(..none, stroke: Some(color.crimson))

pub const stroke_cyan = Style(..none, stroke: Some(color.cyan))

pub const stroke_darkblue = Style(..none, stroke: Some(color.darkblue))

pub const stroke_darkcyan = Style(..none, stroke: Some(color.darkcyan))

pub const stroke_darkgoldenrod = Style(..none, stroke: Some(color.darkgoldenrod))

pub const stroke_darkgray = Style(..none, stroke: Some(color.darkgray))

pub const stroke_darkgreen = Style(..none, stroke: Some(color.darkgreen))

pub const stroke_darkgrey = Style(..none, stroke: Some(color.darkgrey))

pub const stroke_darkkhaki = Style(..none, stroke: Some(color.darkkhaki))

pub const stroke_darkmagenta = Style(..none, stroke: Some(color.darkmagenta))

pub const stroke_darkolivegreen = Style(
  ..none,
  stroke: Some(color.darkolivegreen),
)

pub const stroke_darkorange = Style(..none, stroke: Some(color.darkorange))

pub const stroke_darkorchid = Style(..none, stroke: Some(color.darkorchid))

pub const stroke_darkred = Style(..none, stroke: Some(color.darkred))

pub const stroke_darksalmon = Style(..none, stroke: Some(color.darksalmon))

pub const stroke_darkseagreen = Style(..none, stroke: Some(color.darkseagreen))

pub const stroke_darkslateblue = Style(..none, stroke: Some(color.darkslateblue))

pub const stroke_darkslategray = Style(..none, stroke: Some(color.darkslategray))

pub const stroke_darkslategrey = Style(..none, stroke: Some(color.darkslategrey))

pub const stroke_darkturquoise = Style(..none, stroke: Some(color.darkturquoise))

pub const stroke_darkviolet = Style(..none, stroke: Some(color.darkviolet))

pub const stroke_deeppink = Style(..none, stroke: Some(color.deeppink))

pub const stroke_deepskyblue = Style(..none, stroke: Some(color.deepskyblue))

pub const stroke_dimgray = Style(..none, stroke: Some(color.dimgray))

pub const stroke_dimgrey = Style(..none, stroke: Some(color.dimgrey))

pub const stroke_dodgerblue = Style(..none, stroke: Some(color.dodgerblue))

pub const stroke_firebrick = Style(..none, stroke: Some(color.firebrick))

pub const stroke_floralwhite = Style(..none, stroke: Some(color.floralwhite))

pub const stroke_forestgreen = Style(..none, stroke: Some(color.forestgreen))

pub const stroke_fuchsia = Style(..none, stroke: Some(color.fuchsia))

pub const stroke_gainsboro = Style(..none, stroke: Some(color.gainsboro))

pub const stroke_ghostwhite = Style(..none, stroke: Some(color.ghostwhite))

pub const stroke_gold = Style(..none, stroke: Some(color.gold))

pub const stroke_goldenrod = Style(..none, stroke: Some(color.goldenrod))

pub const stroke_gray = Style(..none, stroke: Some(color.gray))

pub const stroke_green = Style(..none, stroke: Some(color.green))

pub const stroke_greenyellow = Style(..none, stroke: Some(color.greenyellow))

pub const stroke_grey = Style(..none, stroke: Some(color.grey))

pub const stroke_honeydew = Style(..none, stroke: Some(color.honeydew))

pub const stroke_hotpink = Style(..none, stroke: Some(color.hotpink))

pub const stroke_indianred = Style(..none, stroke: Some(color.indianred))

pub const stroke_indigo = Style(..none, stroke: Some(color.indigo))

pub const stroke_ivory = Style(..none, stroke: Some(color.ivory))

pub const stroke_khaki = Style(..none, stroke: Some(color.khaki))

pub const stroke_lavender = Style(..none, stroke: Some(color.lavender))

pub const stroke_lavenderblush = Style(..none, stroke: Some(color.lavenderblush))

pub const stroke_lawngreen = Style(..none, stroke: Some(color.lawngreen))

pub const stroke_lemonchiffon = Style(..none, stroke: Some(color.lemonchiffon))

pub const stroke_lightblue = Style(..none, stroke: Some(color.lightblue))

pub const stroke_lightcoral = Style(..none, stroke: Some(color.lightcoral))

pub const stroke_lightcyan = Style(..none, stroke: Some(color.lightcyan))

pub const stroke_lightgoldenrodyellow = Style(
  ..none,
  stroke: Some(color.lightgoldenrodyellow),
)

pub const stroke_lightgray = Style(..none, stroke: Some(color.lightgray))

pub const stroke_lightgreen = Style(..none, stroke: Some(color.lightgreen))

pub const stroke_lightgrey = Style(..none, stroke: Some(color.lightgrey))

pub const stroke_lightpink = Style(..none, stroke: Some(color.lightpink))

pub const stroke_lightsalmon = Style(..none, stroke: Some(color.lightsalmon))

pub const stroke_lightseagreen = Style(..none, stroke: Some(color.lightseagreen))

pub const stroke_lightskyblue = Style(..none, stroke: Some(color.lightskyblue))

pub const stroke_lightslategray = Style(
  ..none,
  stroke: Some(color.lightslategray),
)

pub const stroke_lightslategrey = Style(
  ..none,
  stroke: Some(color.lightslategrey),
)

pub const stroke_lightsteelblue = Style(
  ..none,
  stroke: Some(color.lightsteelblue),
)

pub const stroke_lightyellow = Style(..none, stroke: Some(color.lightyellow))

pub const stroke_lime = Style(..none, stroke: Some(color.lime))

pub const stroke_limegreen = Style(..none, stroke: Some(color.limegreen))

pub const stroke_linen = Style(..none, stroke: Some(color.linen))

pub const stroke_magenta = Style(..none, stroke: Some(color.magenta))

pub const stroke_maroon = Style(..none, stroke: Some(color.maroon))

pub const stroke_mediumaquamarine = Style(
  ..none,
  stroke: Some(color.mediumaquamarine),
)

pub const stroke_mediumblue = Style(..none, stroke: Some(color.mediumblue))

pub const stroke_mediumorchid = Style(..none, stroke: Some(color.mediumorchid))

pub const stroke_mediumpurple = Style(..none, stroke: Some(color.mediumpurple))

pub const stroke_mediumseagreen = Style(
  ..none,
  stroke: Some(color.mediumseagreen),
)

pub const stroke_mediumslateblue = Style(
  ..none,
  stroke: Some(color.mediumslateblue),
)

pub const stroke_mediumspringgreen = Style(
  ..none,
  stroke: Some(color.mediumspringgreen),
)

pub const stroke_mediumturquoise = Style(
  ..none,
  stroke: Some(color.mediumturquoise),
)

pub const stroke_mediumvioletred = Style(
  ..none,
  stroke: Some(color.mediumvioletred),
)

pub const stroke_midnightblue = Style(..none, stroke: Some(color.midnightblue))

pub const stroke_mintcream = Style(..none, stroke: Some(color.mintcream))

pub const stroke_mistyrose = Style(..none, stroke: Some(color.mistyrose))

pub const stroke_moccasin = Style(..none, stroke: Some(color.moccasin))

pub const stroke_navajowhite = Style(..none, stroke: Some(color.navajowhite))

pub const stroke_navy = Style(..none, stroke: Some(color.navy))

pub const stroke_oldlace = Style(..none, stroke: Some(color.oldlace))

pub const stroke_olive = Style(..none, stroke: Some(color.olive))

pub const stroke_olivedrab = Style(..none, stroke: Some(color.olivedrab))

pub const stroke_orange = Style(..none, stroke: Some(color.orange))

pub const stroke_orangered = Style(..none, stroke: Some(color.orangered))

pub const stroke_orchid = Style(..none, stroke: Some(color.orchid))

pub const stroke_palegoldenrod = Style(..none, stroke: Some(color.palegoldenrod))

pub const stroke_palegreen = Style(..none, stroke: Some(color.palegreen))

pub const stroke_paleturquoise = Style(..none, stroke: Some(color.paleturquoise))

pub const stroke_palevioletred = Style(..none, stroke: Some(color.palevioletred))

pub const stroke_papayawhip = Style(..none, stroke: Some(color.papayawhip))

pub const stroke_peachpuff = Style(..none, stroke: Some(color.peachpuff))

pub const stroke_peru = Style(..none, stroke: Some(color.peru))

pub const stroke_pink = Style(..none, stroke: Some(color.pink))

pub const stroke_plum = Style(..none, stroke: Some(color.plum))

pub const stroke_powderblue = Style(..none, stroke: Some(color.powderblue))

pub const stroke_purple = Style(..none, stroke: Some(color.purple))

pub const stroke_red = Style(..none, stroke: Some(color.red))

pub const stroke_rosybrown = Style(..none, stroke: Some(color.rosybrown))

pub const stroke_royalblue = Style(..none, stroke: Some(color.royalblue))

pub const stroke_saddlebrown = Style(..none, stroke: Some(color.saddlebrown))

pub const stroke_salmon = Style(..none, stroke: Some(color.salmon))

pub const stroke_sandybrown = Style(..none, stroke: Some(color.sandybrown))

pub const stroke_seagreen = Style(..none, stroke: Some(color.seagreen))

pub const stroke_seashell = Style(..none, stroke: Some(color.seashell))

pub const stroke_sienna = Style(..none, stroke: Some(color.sienna))

pub const stroke_silver = Style(..none, stroke: Some(color.silver))

pub const stroke_skyblue = Style(..none, stroke: Some(color.skyblue))

pub const stroke_slateblue = Style(..none, stroke: Some(color.slateblue))

pub const stroke_slategray = Style(..none, stroke: Some(color.slategray))

pub const stroke_slategrey = Style(..none, stroke: Some(color.slategrey))

pub const stroke_snow = Style(..none, stroke: Some(color.snow))

pub const stroke_springgreen = Style(..none, stroke: Some(color.springgreen))

pub const stroke_steelblue = Style(..none, stroke: Some(color.steelblue))

pub const stroke_tan = Style(..none, stroke: Some(color.tan))

pub const stroke_teal = Style(..none, stroke: Some(color.teal))

pub const stroke_thistle = Style(..none, stroke: Some(color.thistle))

pub const stroke_tomato = Style(..none, stroke: Some(color.tomato))

pub const stroke_turquoise = Style(..none, stroke: Some(color.turquoise))

pub const stroke_violet = Style(..none, stroke: Some(color.violet))

pub const stroke_wheat = Style(..none, stroke: Some(color.wheat))

pub const stroke_white = Style(..none, stroke: Some(color.white))

pub const stroke_whitesmoke = Style(..none, stroke: Some(color.whitesmoke))

pub const stroke_yellow = Style(..none, stroke: Some(color.yellow))

pub const stroke_yellowgreen = Style(..none, stroke: Some(color.yellowgreen))
