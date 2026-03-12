import sgleam/color.{type Color}
import sgleam/style.{type Style}

pub const rgb = style.stroke_rgb

pub const rgba = style.stroke_rgba

pub fn with(color: Color) -> Style {
  style.stroke(color)
}

pub const width = style.stroke_width

pub const widthf = style.stroke_widthf

pub const opacity = style.stroke_opacity

pub const opacityf = style.stroke_opacityf

pub const dash_array = style.stroke_dash_array

pub const linecap_butt = style.stroke_linecap_butt

pub const linecap_round = style.stroke_linecap_round

pub const linecap_square = style.stroke_linecap_square

pub const linejoin_bevel = style.stroke_linejoin_bevel

pub const linejoin_miter = style.stroke_linejoin_miter

pub const linejoin_round = style.stroke_linejoin_round

pub const none = style.stroke_none

pub const aliceblue = style.stroke_aliceblue

pub const antiquewhite = style.stroke_antiquewhite

pub const aqua = style.stroke_aqua

pub const aquamarine = style.stroke_aquamarine

pub const azure = style.stroke_azure

pub const beige = style.stroke_beige

pub const bisque = style.stroke_bisque

pub const black = style.stroke_black

pub const blanchedalmond = style.stroke_blanchedalmond

pub const blue = style.stroke_blue

pub const blueviolet = style.stroke_blueviolet

pub const brown = style.stroke_brown

pub const burlywood = style.stroke_burlywood

pub const cadetblue = style.stroke_cadetblue

pub const chartreuse = style.stroke_chartreuse

pub const chocolate = style.stroke_chocolate

pub const coral = style.stroke_coral

pub const cornflowerblue = style.stroke_cornflowerblue

pub const cornsilk = style.stroke_cornsilk

pub const crimson = style.stroke_crimson

pub const cyan = style.stroke_cyan

pub const darkblue = style.stroke_darkblue

pub const darkcyan = style.stroke_darkcyan

pub const darkgoldenrod = style.stroke_darkgoldenrod

pub const darkgray = style.stroke_darkgray

pub const darkgreen = style.stroke_darkgreen

pub const darkgrey = style.stroke_darkgrey

pub const darkkhaki = style.stroke_darkkhaki

pub const darkmagenta = style.stroke_darkmagenta

pub const darkolivegreen = style.stroke_darkolivegreen

pub const darkorange = style.stroke_darkorange

pub const darkorchid = style.stroke_darkorchid

pub const darkred = style.stroke_darkred

pub const darksalmon = style.stroke_darksalmon

pub const darkseagreen = style.stroke_darkseagreen

pub const darkslateblue = style.stroke_darkslateblue

pub const darkslategray = style.stroke_darkslategray

pub const darkslategrey = style.stroke_darkslategrey

pub const darkturquoise = style.stroke_darkturquoise

pub const darkviolet = style.stroke_darkviolet

pub const deeppink = style.stroke_deeppink

pub const deepskyblue = style.stroke_deepskyblue

pub const dimgray = style.stroke_dimgray

pub const dimgrey = style.stroke_dimgrey

pub const dodgerblue = style.stroke_dodgerblue

pub const firebrick = style.stroke_firebrick

pub const floralwhite = style.stroke_floralwhite

pub const forestgreen = style.stroke_forestgreen

pub const fuchsia = style.stroke_fuchsia

pub const gainsboro = style.stroke_gainsboro

pub const ghostwhite = style.stroke_ghostwhite

pub const gold = style.stroke_gold

pub const goldenrod = style.stroke_goldenrod

pub const gray = style.stroke_gray

pub const green = style.stroke_green

pub const greenyellow = style.stroke_greenyellow

pub const grey = style.stroke_grey

pub const honeydew = style.stroke_honeydew

pub const hotpink = style.stroke_hotpink

pub const indianred = style.stroke_indianred

pub const indigo = style.stroke_indigo

pub const ivory = style.stroke_ivory

pub const khaki = style.stroke_khaki

pub const lavender = style.stroke_lavender

pub const lavenderblush = style.stroke_lavenderblush

pub const lawngreen = style.stroke_lawngreen

pub const lemonchiffon = style.stroke_lemonchiffon

pub const lightblue = style.stroke_lightblue

pub const lightcoral = style.stroke_lightcoral

pub const lightcyan = style.stroke_lightcyan

pub const lightgoldenrodyellow = style.stroke_lightgoldenrodyellow

pub const lightgray = style.stroke_lightgray

pub const lightgreen = style.stroke_lightgreen

pub const lightgrey = style.stroke_lightgrey

pub const lightpink = style.stroke_lightpink

pub const lightsalmon = style.stroke_lightsalmon

pub const lightseagreen = style.stroke_lightseagreen

pub const lightskyblue = style.stroke_lightskyblue

pub const lightslategray = style.stroke_lightslategray

pub const lightslategrey = style.stroke_lightslategrey

pub const lightsteelblue = style.stroke_lightsteelblue

pub const lightyellow = style.stroke_lightyellow

pub const lime = style.stroke_lime

pub const limegreen = style.stroke_limegreen

pub const linen = style.stroke_linen

pub const magenta = style.stroke_magenta

pub const maroon = style.stroke_maroon

pub const mediumaquamarine = style.stroke_mediumaquamarine

pub const mediumblue = style.stroke_mediumblue

pub const mediumorchid = style.stroke_mediumorchid

pub const mediumpurple = style.stroke_mediumpurple

pub const mediumseagreen = style.stroke_mediumseagreen

pub const mediumslateblue = style.stroke_mediumslateblue

pub const mediumspringgreen = style.stroke_mediumspringgreen

pub const mediumturquoise = style.stroke_mediumturquoise

pub const mediumvioletred = style.stroke_mediumvioletred

pub const midnightblue = style.stroke_midnightblue

pub const mintcream = style.stroke_mintcream

pub const mistyrose = style.stroke_mistyrose

pub const moccasin = style.stroke_moccasin

pub const navajowhite = style.stroke_navajowhite

pub const navy = style.stroke_navy

pub const oldlace = style.stroke_oldlace

pub const olive = style.stroke_olive

pub const olivedrab = style.stroke_olivedrab

pub const orange = style.stroke_orange

pub const orangered = style.stroke_orangered

pub const orchid = style.stroke_orchid

pub const palegoldenrod = style.stroke_palegoldenrod

pub const palegreen = style.stroke_palegreen

pub const paleturquoise = style.stroke_paleturquoise

pub const palevioletred = style.stroke_palevioletred

pub const papayawhip = style.stroke_papayawhip

pub const peachpuff = style.stroke_peachpuff

pub const peru = style.stroke_peru

pub const pink = style.stroke_pink

pub const plum = style.stroke_plum

pub const powderblue = style.stroke_powderblue

pub const purple = style.stroke_purple

pub const red = style.stroke_red

pub const rosybrown = style.stroke_rosybrown

pub const royalblue = style.stroke_royalblue

pub const saddlebrown = style.stroke_saddlebrown

pub const salmon = style.stroke_salmon

pub const sandybrown = style.stroke_sandybrown

pub const seagreen = style.stroke_seagreen

pub const seashell = style.stroke_seashell

pub const sienna = style.stroke_sienna

pub const silver = style.stroke_silver

pub const skyblue = style.stroke_skyblue

pub const slateblue = style.stroke_slateblue

pub const slategray = style.stroke_slategray

pub const slategrey = style.stroke_slategrey

pub const snow = style.stroke_snow

pub const springgreen = style.stroke_springgreen

pub const steelblue = style.stroke_steelblue

pub const tan = style.stroke_tan

pub const teal = style.stroke_teal

pub const thistle = style.stroke_thistle

pub const tomato = style.stroke_tomato

pub const turquoise = style.stroke_turquoise

pub const violet = style.stroke_violet

pub const wheat = style.stroke_wheat

pub const white = style.stroke_white

pub const whitesmoke = style.stroke_whitesmoke

pub const yellow = style.stroke_yellow

pub const yellowgreen = style.stroke_yellowgreen
