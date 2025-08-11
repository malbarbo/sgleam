import gleam/float
import gleam/int
import gleam/list
import gleam/result
import gleam/string
import sgleam/color.{type Color}

pub opaque type Style {
  Style(attributes: List(Attribute))
}

type Attribute {
  Fill(Color)
  FillOpacity(Float)
  FillRule(String)
  Stroke(Color)
  StokeLineCap(String)
  StokeLineJoin(String)
  StrokeWidth(Float)
  StrokeOpacity(Float)
  StrokeDashArray(List(Int))
}

pub fn join(style: List(Style)) -> Style {
  style
  |> list.flat_map(fn(s) { s.attributes })
  |> Style
}

pub const none = Style([])

pub fn fill(c: Color) -> Style {
  Style([Fill(c)])
}

pub fn fill_opacity(opacity: Float) -> Style {
  Style([FillOpacity(float.clamp(opacity, 0.0, 1.0))])
}

pub fn stroke(c: Color) -> Style {
  Style([Stroke(c)])
}

pub fn stroke_width(width: Float) -> Style {
  Style([StrokeWidth(positive(width))])
}

pub fn stroke_opacity(opacity: Float) -> Style {
  Style([StrokeOpacity(float.clamp(opacity, 0.0, 1.0))])
}

pub fn stroke_dash_array(values: List(Int)) -> Style {
  Style([StrokeDashArray(list.map(values, int.max(0, _)))])
}

pub fn to_svg(style: Style) -> String {
  case has_outline(style) && !has_fill(style) {
    False -> style.attributes
    True -> [Fill(color.none), ..style.attributes]
  }
  |> list.map(attribute_to_svg)
  |> string.join("")
}

fn has_fill(style: Style) -> Bool {
  style.attributes
  |> list.find(fn(s) {
    case s {
      Fill(_) -> True
      _ -> False
    }
  })
  |> result.is_ok
}

fn has_outline(style: Style) -> Bool {
  style.attributes
  |> list.find(fn(s) {
    case s {
      Stroke(_) -> True
      _ -> False
    }
  })
  |> result.is_ok
}

fn attribute_to_svg(style: Attribute) -> String {
  case style {
    Fill(c) -> attribs("fill", color.to_svg(c))
    FillOpacity(v) -> attrib("fill-opacity", v)
    FillRule(v) -> attribs("fill-rule", v)
    Stroke(c) -> attribs("stroke", color.to_svg(c))
    StokeLineCap(s) -> attribs("stroke-linecap", s)
    StokeLineJoin(s) -> attribs("stroke-linejoin", s)
    StrokeDashArray(values) ->
      attribs(
        "stroke-dasharray",
        values |> list.map(int.to_string) |> string.join(", "),
      )
    StrokeOpacity(v) -> attrib("stroke-opacity", v)
    StrokeWidth(v) -> attrib("stroke-width", v)
  }
}

fn positive(n: Float) -> Float {
  float.max(0.0, n)
}

fn attrib(name: String, value: Float) -> String {
  name <> "=\"" <> float.to_string(value) <> "\" "
}

fn attribs(name: String, value: String) -> String {
  name <> "=\"" <> value <> "\" "
}

// **************************
// * Fill
// **************************

pub const fill_rule_nonzero = Style([FillRule("nonzero")])

pub const fill_rule_evenodd = Style([FillRule("evenodd")])

pub fn fill_rgb(red: Int, green: Int, blue: Int) -> Style {
  Style([Fill(color.rgb(red, green, blue))])
}

pub fn fill_rgba(red: Int, green: Int, blue: Int, alpha: Float) -> Style {
  Style([Fill(color.rgba(red, green, blue, alpha))])
}

pub const fill_none = Style([Fill(color.none)])

pub const fill_aliceblue = Style([Fill(color.aliceblue)])

pub const fill_antiquewhite = Style([Fill(color.antiquewhite)])

pub const fill_aqua = Style([Fill(color.aqua)])

pub const fill_aquamarine = Style([Fill(color.aquamarine)])

pub const fill_azure = Style([Fill(color.azure)])

pub const fill_beige = Style([Fill(color.beige)])

pub const fill_bisque = Style([Fill(color.bisque)])

pub const fill_black = Style([Fill(color.black)])

pub const fill_blanchedalmond = Style([Fill(color.blanchedalmond)])

pub const fill_blue = Style([Fill(color.blue)])

pub const fill_blueviolet = Style([Fill(color.blueviolet)])

pub const fill_brown = Style([Fill(color.brown)])

pub const fill_burlywood = Style([Fill(color.burlywood)])

pub const fill_cadetblue = Style([Fill(color.cadetblue)])

pub const fill_chartreuse = Style([Fill(color.chartreuse)])

pub const fill_chocolate = Style([Fill(color.chocolate)])

pub const fill_coral = Style([Fill(color.coral)])

pub const fill_cornflowerblue = Style([Fill(color.cornflowerblue)])

pub const fill_cornsilk = Style([Fill(color.cornsilk)])

pub const fill_crimson = Style([Fill(color.crimson)])

pub const fill_cyan = Style([Fill(color.cyan)])

pub const fill_darkblue = Style([Fill(color.darkblue)])

pub const fill_darkcyan = Style([Fill(color.darkcyan)])

pub const fill_darkgoldenrod = Style([Fill(color.darkgoldenrod)])

pub const fill_darkgray = Style([Fill(color.darkgray)])

pub const fill_darkgreen = Style([Fill(color.darkgreen)])

pub const fill_darkgrey = Style([Fill(color.darkgrey)])

pub const fill_darkkhaki = Style([Fill(color.darkkhaki)])

pub const fill_darkmagenta = Style([Fill(color.darkmagenta)])

pub const fill_darkolivegreen = Style([Fill(color.darkolivegreen)])

pub const fill_darkorange = Style([Fill(color.darkorange)])

pub const fill_darkorchid = Style([Fill(color.darkorchid)])

pub const fill_darkred = Style([Fill(color.darkred)])

pub const fill_darksalmon = Style([Fill(color.darksalmon)])

pub const fill_darkseagreen = Style([Fill(color.darkseagreen)])

pub const fill_darkslateblue = Style([Fill(color.darkslateblue)])

pub const fill_darkslategray = Style([Fill(color.darkslategray)])

pub const fill_darkslategrey = Style([Fill(color.darkslategrey)])

pub const fill_darkturquoise = Style([Fill(color.darkturquoise)])

pub const fill_darkviolet = Style([Fill(color.darkviolet)])

pub const fill_deeppink = Style([Fill(color.deeppink)])

pub const fill_deepskyblue = Style([Fill(color.deepskyblue)])

pub const fill_dimgray = Style([Fill(color.dimgray)])

pub const fill_dimgrey = Style([Fill(color.dimgrey)])

pub const fill_dodgerblue = Style([Fill(color.dodgerblue)])

pub const fill_firebrick = Style([Fill(color.firebrick)])

pub const fill_floralwhite = Style([Fill(color.floralwhite)])

pub const fill_forestgreen = Style([Fill(color.forestgreen)])

pub const fill_fuchsia = Style([Fill(color.fuchsia)])

pub const fill_gainsboro = Style([Fill(color.gainsboro)])

pub const fill_ghostwhite = Style([Fill(color.ghostwhite)])

pub const fill_gold = Style([Fill(color.gold)])

pub const fill_goldenrod = Style([Fill(color.goldenrod)])

pub const fill_gray = Style([Fill(color.gray)])

pub const fill_green = Style([Fill(color.green)])

pub const fill_greenyellow = Style([Fill(color.greenyellow)])

pub const fill_grey = Style([Fill(color.grey)])

pub const fill_honeydew = Style([Fill(color.honeydew)])

pub const fill_hotpink = Style([Fill(color.hotpink)])

pub const fill_indianred = Style([Fill(color.indianred)])

pub const fill_indigo = Style([Fill(color.indigo)])

pub const fill_ivory = Style([Fill(color.ivory)])

pub const fill_khaki = Style([Fill(color.khaki)])

pub const fill_lavender = Style([Fill(color.lavender)])

pub const fill_lavenderblush = Style([Fill(color.lavenderblush)])

pub const fill_lawngreen = Style([Fill(color.lawngreen)])

pub const fill_lemonchiffon = Style([Fill(color.lemonchiffon)])

pub const fill_lightblue = Style([Fill(color.lightblue)])

pub const fill_lightcoral = Style([Fill(color.lightcoral)])

pub const fill_lightcyan = Style([Fill(color.lightcyan)])

pub const fill_lightgoldenrodyellow = Style([Fill(color.lightgoldenrodyellow)])

pub const fill_lightgray = Style([Fill(color.lightgray)])

pub const fill_lightgreen = Style([Fill(color.lightgreen)])

pub const fill_lightgrey = Style([Fill(color.lightgrey)])

pub const fill_lightpink = Style([Fill(color.lightpink)])

pub const fill_lightsalmon = Style([Fill(color.lightsalmon)])

pub const fill_lightseagreen = Style([Fill(color.lightseagreen)])

pub const fill_lightskyblue = Style([Fill(color.lightskyblue)])

pub const fill_lightslategray = Style([Fill(color.lightslategray)])

pub const fill_lightslategrey = Style([Fill(color.lightslategrey)])

pub const fill_lightsteelblue = Style([Fill(color.lightsteelblue)])

pub const fill_lightyellow = Style([Fill(color.lightyellow)])

pub const fill_lime = Style([Fill(color.lime)])

pub const fill_limegreen = Style([Fill(color.limegreen)])

pub const fill_linen = Style([Fill(color.linen)])

pub const fill_magenta = Style([Fill(color.magenta)])

pub const fill_maroon = Style([Fill(color.maroon)])

pub const fill_mediumaquamarine = Style([Fill(color.mediumaquamarine)])

pub const fill_mediumblue = Style([Fill(color.mediumblue)])

pub const fill_mediumorchid = Style([Fill(color.mediumorchid)])

pub const fill_mediumpurple = Style([Fill(color.mediumpurple)])

pub const fill_mediumseagreen = Style([Fill(color.mediumseagreen)])

pub const fill_mediumslateblue = Style([Fill(color.mediumslateblue)])

pub const fill_mediumspringgreen = Style([Fill(color.mediumspringgreen)])

pub const fill_mediumturquoise = Style([Fill(color.mediumturquoise)])

pub const fill_mediumvioletred = Style([Fill(color.mediumvioletred)])

pub const fill_midnightblue = Style([Fill(color.midnightblue)])

pub const fill_mintcream = Style([Fill(color.mintcream)])

pub const fill_mistyrose = Style([Fill(color.mistyrose)])

pub const fill_moccasin = Style([Fill(color.moccasin)])

pub const fill_navajowhite = Style([Fill(color.navajowhite)])

pub const fill_navy = Style([Fill(color.navy)])

pub const fill_oldlace = Style([Fill(color.oldlace)])

pub const fill_olive = Style([Fill(color.olive)])

pub const fill_olivedrab = Style([Fill(color.olivedrab)])

pub const fill_orange = Style([Fill(color.orange)])

pub const fill_orangered = Style([Fill(color.orangered)])

pub const fill_orchid = Style([Fill(color.orchid)])

pub const fill_palegoldenrod = Style([Fill(color.palegoldenrod)])

pub const fill_palegreen = Style([Fill(color.palegreen)])

pub const fill_paleturquoise = Style([Fill(color.paleturquoise)])

pub const fill_palevioletred = Style([Fill(color.palevioletred)])

pub const fill_papayawhip = Style([Fill(color.papayawhip)])

pub const fill_peachpuff = Style([Fill(color.peachpuff)])

pub const fill_peru = Style([Fill(color.peru)])

pub const fill_pink = Style([Fill(color.pink)])

pub const fill_plum = Style([Fill(color.plum)])

pub const fill_powderblue = Style([Fill(color.powderblue)])

pub const fill_purple = Style([Fill(color.purple)])

pub const fill_red = Style([Fill(color.red)])

pub const fill_rosybrown = Style([Fill(color.rosybrown)])

pub const fill_royalblue = Style([Fill(color.royalblue)])

pub const fill_saddlebrown = Style([Fill(color.saddlebrown)])

pub const fill_salmon = Style([Fill(color.salmon)])

pub const fill_sandybrown = Style([Fill(color.sandybrown)])

pub const fill_seagreen = Style([Fill(color.seagreen)])

pub const fill_seashell = Style([Fill(color.seashell)])

pub const fill_sienna = Style([Fill(color.sienna)])

pub const fill_silver = Style([Fill(color.silver)])

pub const fill_skyblue = Style([Fill(color.skyblue)])

pub const fill_slateblue = Style([Fill(color.slateblue)])

pub const fill_slategray = Style([Fill(color.slategray)])

pub const fill_slategrey = Style([Fill(color.slategrey)])

pub const fill_snow = Style([Fill(color.snow)])

pub const fill_springgreen = Style([Fill(color.springgreen)])

pub const fill_steelblue = Style([Fill(color.steelblue)])

pub const fill_tan = Style([Fill(color.tan)])

pub const fill_teal = Style([Fill(color.teal)])

pub const fill_thistle = Style([Fill(color.thistle)])

pub const fill_tomato = Style([Fill(color.tomato)])

pub const fill_turquoise = Style([Fill(color.turquoise)])

pub const fill_violet = Style([Fill(color.violet)])

pub const fill_wheat = Style([Fill(color.wheat)])

pub const fill_white = Style([Fill(color.white)])

pub const fill_whitesmoke = Style([Fill(color.whitesmoke)])

pub const fill_yellow = Style([Fill(color.yellow)])

pub const fill_yellowgreen = Style([Fill(color.yellowgreen)])

// **************************
// * Stroke
// **************************

pub const stroke_linecap_butt = Style([StokeLineCap("butt")])

pub const stroke_linecap_round = Style([StokeLineCap("round")])

pub const stroke_linecap_square = Style([StokeLineCap("square")])

pub const stroke_linejoin_bevel = Style([StokeLineJoin("bevel")])

pub const stroke_linejoin_miter = Style([StokeLineJoin("miter")])

pub const stroke_linejoin_round = Style([StokeLineJoin("round")])

pub fn stroke_rgb(red: Int, green: Int, blue: Int) -> Style {
  Style([Stroke(color.rgb(red, green, blue))])
}

pub fn stroke_rgba(red: Int, green: Int, blue: Int, alpha: Float) -> Style {
  Style([Stroke(color.rgba(red, green, blue, alpha))])
}

pub const stroke_none = Style([Stroke(color.none)])

pub const stroke_aliceblue = Style([Stroke(color.aliceblue)])

pub const stroke_antiquewhite = Style([Stroke(color.antiquewhite)])

pub const stroke_aqua = Style([Stroke(color.aqua)])

pub const stroke_aquamarine = Style([Stroke(color.aquamarine)])

pub const stroke_azure = Style([Stroke(color.azure)])

pub const stroke_beige = Style([Stroke(color.beige)])

pub const stroke_bisque = Style([Stroke(color.bisque)])

pub const stroke_black = Style([Stroke(color.black)])

pub const stroke_blanchedalmond = Style([Stroke(color.blanchedalmond)])

pub const stroke_blue = Style([Stroke(color.blue)])

pub const stroke_blueviolet = Style([Stroke(color.blueviolet)])

pub const stroke_brown = Style([Stroke(color.brown)])

pub const stroke_burlywood = Style([Stroke(color.burlywood)])

pub const stroke_cadetblue = Style([Stroke(color.cadetblue)])

pub const stroke_chartreuse = Style([Stroke(color.chartreuse)])

pub const stroke_chocolate = Style([Stroke(color.chocolate)])

pub const stroke_coral = Style([Stroke(color.coral)])

pub const stroke_cornflowerblue = Style([Stroke(color.cornflowerblue)])

pub const stroke_cornsilk = Style([Stroke(color.cornsilk)])

pub const stroke_crimson = Style([Stroke(color.crimson)])

pub const stroke_cyan = Style([Stroke(color.cyan)])

pub const stroke_darkblue = Style([Stroke(color.darkblue)])

pub const stroke_darkcyan = Style([Stroke(color.darkcyan)])

pub const stroke_darkgoldenrod = Style([Stroke(color.darkgoldenrod)])

pub const stroke_darkgray = Style([Stroke(color.darkgray)])

pub const stroke_darkgreen = Style([Stroke(color.darkgreen)])

pub const stroke_darkgrey = Style([Stroke(color.darkgrey)])

pub const stroke_darkkhaki = Style([Stroke(color.darkkhaki)])

pub const stroke_darkmagenta = Style([Stroke(color.darkmagenta)])

pub const stroke_darkolivegreen = Style([Stroke(color.darkolivegreen)])

pub const stroke_darkorange = Style([Stroke(color.darkorange)])

pub const stroke_darkorchid = Style([Stroke(color.darkorchid)])

pub const stroke_darkred = Style([Stroke(color.darkred)])

pub const stroke_darksalmon = Style([Stroke(color.darksalmon)])

pub const stroke_darkseagreen = Style([Stroke(color.darkseagreen)])

pub const stroke_darkslateblue = Style([Stroke(color.darkslateblue)])

pub const stroke_darkslategray = Style([Stroke(color.darkslategray)])

pub const stroke_darkslategrey = Style([Stroke(color.darkslategrey)])

pub const stroke_darkturquoise = Style([Stroke(color.darkturquoise)])

pub const stroke_darkviolet = Style([Stroke(color.darkviolet)])

pub const stroke_deeppink = Style([Stroke(color.deeppink)])

pub const stroke_deepskyblue = Style([Stroke(color.deepskyblue)])

pub const stroke_dimgray = Style([Stroke(color.dimgray)])

pub const stroke_dimgrey = Style([Stroke(color.dimgrey)])

pub const stroke_dodgerblue = Style([Stroke(color.dodgerblue)])

pub const stroke_firebrick = Style([Stroke(color.firebrick)])

pub const stroke_floralwhite = Style([Stroke(color.floralwhite)])

pub const stroke_forestgreen = Style([Stroke(color.forestgreen)])

pub const stroke_fuchsia = Style([Stroke(color.fuchsia)])

pub const stroke_gainsboro = Style([Stroke(color.gainsboro)])

pub const stroke_ghostwhite = Style([Stroke(color.ghostwhite)])

pub const stroke_gold = Style([Stroke(color.gold)])

pub const stroke_goldenrod = Style([Stroke(color.goldenrod)])

pub const stroke_gray = Style([Stroke(color.gray)])

pub const stroke_green = Style([Stroke(color.green)])

pub const stroke_greenyellow = Style([Stroke(color.greenyellow)])

pub const stroke_grey = Style([Stroke(color.grey)])

pub const stroke_honeydew = Style([Stroke(color.honeydew)])

pub const stroke_hotpink = Style([Stroke(color.hotpink)])

pub const stroke_indianred = Style([Stroke(color.indianred)])

pub const stroke_indigo = Style([Stroke(color.indigo)])

pub const stroke_ivory = Style([Stroke(color.ivory)])

pub const stroke_khaki = Style([Stroke(color.khaki)])

pub const stroke_lavender = Style([Stroke(color.lavender)])

pub const stroke_lavenderblush = Style([Stroke(color.lavenderblush)])

pub const stroke_lawngreen = Style([Stroke(color.lawngreen)])

pub const stroke_lemonchiffon = Style([Stroke(color.lemonchiffon)])

pub const stroke_lightblue = Style([Stroke(color.lightblue)])

pub const stroke_lightcoral = Style([Stroke(color.lightcoral)])

pub const stroke_lightcyan = Style([Stroke(color.lightcyan)])

pub const stroke_lightgoldenrodyellow = Style(
  [Stroke(color.lightgoldenrodyellow)],
)

pub const stroke_lightgray = Style([Stroke(color.lightgray)])

pub const stroke_lightgreen = Style([Stroke(color.lightgreen)])

pub const stroke_lightgrey = Style([Stroke(color.lightgrey)])

pub const stroke_lightpink = Style([Stroke(color.lightpink)])

pub const stroke_lightsalmon = Style([Stroke(color.lightsalmon)])

pub const stroke_lightseagreen = Style([Stroke(color.lightseagreen)])

pub const stroke_lightskyblue = Style([Stroke(color.lightskyblue)])

pub const stroke_lightslategray = Style([Stroke(color.lightslategray)])

pub const stroke_lightslategrey = Style([Stroke(color.lightslategrey)])

pub const stroke_lightsteelblue = Style([Stroke(color.lightsteelblue)])

pub const stroke_lightyellow = Style([Stroke(color.lightyellow)])

pub const stroke_lime = Style([Stroke(color.lime)])

pub const stroke_limegreen = Style([Stroke(color.limegreen)])

pub const stroke_linen = Style([Stroke(color.linen)])

pub const stroke_magenta = Style([Stroke(color.magenta)])

pub const stroke_maroon = Style([Stroke(color.maroon)])

pub const stroke_mediumaquamarine = Style([Stroke(color.mediumaquamarine)])

pub const stroke_mediumblue = Style([Stroke(color.mediumblue)])

pub const stroke_mediumorchid = Style([Stroke(color.mediumorchid)])

pub const stroke_mediumpurple = Style([Stroke(color.mediumpurple)])

pub const stroke_mediumseagreen = Style([Stroke(color.mediumseagreen)])

pub const stroke_mediumslateblue = Style([Stroke(color.mediumslateblue)])

pub const stroke_mediumspringgreen = Style([Stroke(color.mediumspringgreen)])

pub const stroke_mediumturquoise = Style([Stroke(color.mediumturquoise)])

pub const stroke_mediumvioletred = Style([Stroke(color.mediumvioletred)])

pub const stroke_midnightblue = Style([Stroke(color.midnightblue)])

pub const stroke_mintcream = Style([Stroke(color.mintcream)])

pub const stroke_mistyrose = Style([Stroke(color.mistyrose)])

pub const stroke_moccasin = Style([Stroke(color.moccasin)])

pub const stroke_navajowhite = Style([Stroke(color.navajowhite)])

pub const stroke_navy = Style([Stroke(color.navy)])

pub const stroke_oldlace = Style([Stroke(color.oldlace)])

pub const stroke_olive = Style([Stroke(color.olive)])

pub const stroke_olivedrab = Style([Stroke(color.olivedrab)])

pub const stroke_orange = Style([Stroke(color.orange)])

pub const stroke_orangered = Style([Stroke(color.orangered)])

pub const stroke_orchid = Style([Stroke(color.orchid)])

pub const stroke_palegoldenrod = Style([Stroke(color.palegoldenrod)])

pub const stroke_palegreen = Style([Stroke(color.palegreen)])

pub const stroke_paleturquoise = Style([Stroke(color.paleturquoise)])

pub const stroke_palevioletred = Style([Stroke(color.palevioletred)])

pub const stroke_papayawhip = Style([Stroke(color.papayawhip)])

pub const stroke_peachpuff = Style([Stroke(color.peachpuff)])

pub const stroke_peru = Style([Stroke(color.peru)])

pub const stroke_pink = Style([Stroke(color.pink)])

pub const stroke_plum = Style([Stroke(color.plum)])

pub const stroke_powderblue = Style([Stroke(color.powderblue)])

pub const stroke_purple = Style([Stroke(color.purple)])

pub const stroke_red = Style([Stroke(color.red)])

pub const stroke_rosybrown = Style([Stroke(color.rosybrown)])

pub const stroke_royalblue = Style([Stroke(color.royalblue)])

pub const stroke_saddlebrown = Style([Stroke(color.saddlebrown)])

pub const stroke_salmon = Style([Stroke(color.salmon)])

pub const stroke_sandybrown = Style([Stroke(color.sandybrown)])

pub const stroke_seagreen = Style([Stroke(color.seagreen)])

pub const stroke_seashell = Style([Stroke(color.seashell)])

pub const stroke_sienna = Style([Stroke(color.sienna)])

pub const stroke_silver = Style([Stroke(color.silver)])

pub const stroke_skyblue = Style([Stroke(color.skyblue)])

pub const stroke_slateblue = Style([Stroke(color.slateblue)])

pub const stroke_slategray = Style([Stroke(color.slategray)])

pub const stroke_slategrey = Style([Stroke(color.slategrey)])

pub const stroke_snow = Style([Stroke(color.snow)])

pub const stroke_springgreen = Style([Stroke(color.springgreen)])

pub const stroke_steelblue = Style([Stroke(color.steelblue)])

pub const stroke_tan = Style([Stroke(color.tan)])

pub const stroke_teal = Style([Stroke(color.teal)])

pub const stroke_thistle = Style([Stroke(color.thistle)])

pub const stroke_tomato = Style([Stroke(color.tomato)])

pub const stroke_turquoise = Style([Stroke(color.turquoise)])

pub const stroke_violet = Style([Stroke(color.violet)])

pub const stroke_wheat = Style([Stroke(color.wheat)])

pub const stroke_white = Style([Stroke(color.white)])

pub const stroke_whitesmoke = Style([Stroke(color.whitesmoke)])

pub const stroke_yellow = Style([Stroke(color.yellow)])

pub const stroke_yellowgreen = Style([Stroke(color.yellowgreen)])
