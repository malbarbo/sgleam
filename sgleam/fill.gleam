import sgleam/color.{type Color}
import sgleam/style.{type Style}

pub const rgb = style.fill_rgb

pub const rgba = style.fill_rgba

pub const opacity = style.fill_opacity

pub fn with(color: Color) -> Style {
  style.fill(color)
}

pub const rule_nonzero = style.fill_rule_nonzero

pub const rule_evenodd = style.fill_rule_evenodd

pub const none = style.fill_none

pub const aliceblue = style.fill_aliceblue

pub const antiquewhite = style.fill_antiquewhite

pub const aqua = style.fill_aqua

pub const aquamarine = style.fill_aquamarine

pub const azure = style.fill_azure

pub const beige = style.fill_beige

pub const bisque = style.fill_bisque

pub const black = style.fill_black

pub const blanchedalmond = style.fill_blanchedalmond

pub const blue = style.fill_blue

pub const blueviolet = style.fill_blueviolet

pub const brown = style.fill_brown

pub const burlywood = style.fill_burlywood

pub const cadetblue = style.fill_cadetblue

pub const chartreuse = style.fill_chartreuse

pub const chocolate = style.fill_chocolate

pub const coral = style.fill_coral

pub const cornflowerblue = style.fill_cornflowerblue

pub const cornsilk = style.fill_cornsilk

pub const crimson = style.fill_crimson

pub const cyan = style.fill_cyan

pub const darkblue = style.fill_darkblue

pub const darkcyan = style.fill_darkcyan

pub const darkgoldenrod = style.fill_darkgoldenrod

pub const darkgray = style.fill_darkgray

pub const darkgreen = style.fill_darkgreen

pub const darkgrey = style.fill_darkgrey

pub const darkkhaki = style.fill_darkkhaki

pub const darkmagenta = style.fill_darkmagenta

pub const darkolivegreen = style.fill_darkolivegreen

pub const darkorange = style.fill_darkorange

pub const darkorchid = style.fill_darkorchid

pub const darkred = style.fill_darkred

pub const darksalmon = style.fill_darksalmon

pub const darkseagreen = style.fill_darkseagreen

pub const darkslateblue = style.fill_darkslateblue

pub const darkslategray = style.fill_darkslategray

pub const darkslategrey = style.fill_darkslategrey

pub const darkturquoise = style.fill_darkturquoise

pub const darkviolet = style.fill_darkviolet

pub const deeppink = style.fill_deeppink

pub const deepskyblue = style.fill_deepskyblue

pub const dimgray = style.fill_dimgray

pub const dimgrey = style.fill_dimgrey

pub const dodgerblue = style.fill_dodgerblue

pub const firebrick = style.fill_firebrick

pub const floralwhite = style.fill_floralwhite

pub const forestgreen = style.fill_forestgreen

pub const fuchsia = style.fill_fuchsia

pub const gainsboro = style.fill_gainsboro

pub const ghostwhite = style.fill_ghostwhite

pub const gold = style.fill_gold

pub const goldenrod = style.fill_goldenrod

pub const gray = style.fill_gray

pub const green = style.fill_green

pub const greenyellow = style.fill_greenyellow

pub const grey = style.fill_grey

pub const honeydew = style.fill_honeydew

pub const hotpink = style.fill_hotpink

pub const indianred = style.fill_indianred

pub const indigo = style.fill_indigo

pub const ivory = style.fill_ivory

pub const khaki = style.fill_khaki

pub const lavender = style.fill_lavender

pub const lavenderblush = style.fill_lavenderblush

pub const lawngreen = style.fill_lawngreen

pub const lemonchiffon = style.fill_lemonchiffon

pub const lightblue = style.fill_lightblue

pub const lightcoral = style.fill_lightcoral

pub const lightcyan = style.fill_lightcyan

pub const lightgoldenrodyellow = style.fill_lightgoldenrodyellow

pub const lightgray = style.fill_lightgray

pub const lightgreen = style.fill_lightgreen

pub const lightgrey = style.fill_lightgrey

pub const lightpink = style.fill_lightpink

pub const lightsalmon = style.fill_lightsalmon

pub const lightseagreen = style.fill_lightseagreen

pub const lightskyblue = style.fill_lightskyblue

pub const lightslategray = style.fill_lightslategray

pub const lightslategrey = style.fill_lightslategrey

pub const lightsteelblue = style.fill_lightsteelblue

pub const lightyellow = style.fill_lightyellow

pub const lime = style.fill_lime

pub const limegreen = style.fill_limegreen

pub const linen = style.fill_linen

pub const magenta = style.fill_magenta

pub const maroon = style.fill_maroon

pub const mediumaquamarine = style.fill_mediumaquamarine

pub const mediumblue = style.fill_mediumblue

pub const mediumorchid = style.fill_mediumorchid

pub const mediumpurple = style.fill_mediumpurple

pub const mediumseagreen = style.fill_mediumseagreen

pub const mediumslateblue = style.fill_mediumslateblue

pub const mediumspringgreen = style.fill_mediumspringgreen

pub const mediumturquoise = style.fill_mediumturquoise

pub const mediumvioletred = style.fill_mediumvioletred

pub const midnightblue = style.fill_midnightblue

pub const mintcream = style.fill_mintcream

pub const mistyrose = style.fill_mistyrose

pub const moccasin = style.fill_moccasin

pub const navajowhite = style.fill_navajowhite

pub const navy = style.fill_navy

pub const oldlace = style.fill_oldlace

pub const olive = style.fill_olive

pub const olivedrab = style.fill_olivedrab

pub const orange = style.fill_orange

pub const orangered = style.fill_orangered

pub const orchid = style.fill_orchid

pub const palegoldenrod = style.fill_palegoldenrod

pub const palegreen = style.fill_palegreen

pub const paleturquoise = style.fill_paleturquoise

pub const palevioletred = style.fill_palevioletred

pub const papayawhip = style.fill_papayawhip

pub const peachpuff = style.fill_peachpuff

pub const peru = style.fill_peru

pub const pink = style.fill_pink

pub const plum = style.fill_plum

pub const powderblue = style.fill_powderblue

pub const purple = style.fill_purple

pub const red = style.fill_red

pub const rosybrown = style.fill_rosybrown

pub const royalblue = style.fill_royalblue

pub const saddlebrown = style.fill_saddlebrown

pub const salmon = style.fill_salmon

pub const sandybrown = style.fill_sandybrown

pub const seagreen = style.fill_seagreen

pub const seashell = style.fill_seashell

pub const sienna = style.fill_sienna

pub const silver = style.fill_silver

pub const skyblue = style.fill_skyblue

pub const slateblue = style.fill_slateblue

pub const slategray = style.fill_slategray

pub const slategrey = style.fill_slategrey

pub const snow = style.fill_snow

pub const springgreen = style.fill_springgreen

pub const steelblue = style.fill_steelblue

pub const tan = style.fill_tan

pub const teal = style.fill_teal

pub const thistle = style.fill_thistle

pub const tomato = style.fill_tomato

pub const turquoise = style.fill_turquoise

pub const violet = style.fill_violet

pub const wheat = style.fill_wheat

pub const white = style.fill_white

pub const whitesmoke = style.fill_whitesmoke

pub const yellow = style.fill_yellow

pub const yellowgreen = style.fill_yellowgreen
