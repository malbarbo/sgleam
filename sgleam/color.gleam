import gleam/float
import gleam/int

pub opaque type Color {
  None
  Rgb(Int, Int, Int, Float)
}

pub fn to_svg(color: Color) -> String {
  case color {
    None -> "none"
    Rgb(r, g, b, a) ->
      "rgba("
      <> int.to_string(r)
      <> ", "
      <> int.to_string(g)
      <> ", "
      <> int.to_string(b)
      <> ", "
      <> float.to_string(a)
      <> ")"
  }
}

pub fn rgb(red: Int, green: Int, blue: Int) -> Color {
  Rgb(
    int.clamp(red, 0, 255),
    int.clamp(green, 0, 255),
    int.clamp(blue, 0, 255),
    1.0,
  )
}

pub fn rgba(red: Int, green: Int, blue: Int, alpha: Float) -> Color {
  Rgb(
    int.clamp(red, 0, 255),
    int.clamp(green, 0, 255),
    int.clamp(blue, 0, 255),
    float.clamp(alpha, 0.0, 1.0),
  )
}

pub const none = None

pub const aliceblue = Rgb(240, 248, 255, 1.0)

pub const antiquewhite = Rgb(250, 235, 215, 1.0)

pub const aqua = Rgb(0, 255, 255, 1.0)

pub const aquamarine = Rgb(127, 255, 212, 1.0)

pub const azure = Rgb(240, 255, 255, 1.0)

pub const beige = Rgb(245, 245, 220, 1.0)

pub const bisque = Rgb(255, 228, 196, 1.0)

pub const black = Rgb(0, 0, 0, 1.0)

pub const blanchedalmond = Rgb(255, 235, 205, 1.0)

pub const blue = Rgb(0, 0, 255, 1.0)

pub const blueviolet = Rgb(138, 43, 226, 1.0)

pub const brown = Rgb(165, 42, 42, 1.0)

pub const burlywood = Rgb(222, 184, 135, 1.0)

pub const cadetblue = Rgb(95, 158, 160, 1.0)

pub const chartreuse = Rgb(127, 255, 0, 1.0)

pub const chocolate = Rgb(210, 105, 30, 1.0)

pub const coral = Rgb(255, 127, 80, 1.0)

pub const cornflowerblue = Rgb(100, 149, 237, 1.0)

pub const cornsilk = Rgb(255, 248, 220, 1.0)

pub const crimson = Rgb(220, 20, 60, 1.0)

pub const cyan = Rgb(0, 255, 255, 1.0)

pub const darkblue = Rgb(0, 0, 139, 1.0)

pub const darkcyan = Rgb(0, 139, 139, 1.0)

pub const darkgoldenrod = Rgb(184, 134, 11, 1.0)

pub const darkgray = Rgb(169, 169, 169, 1.0)

pub const darkgreen = Rgb(0, 100, 0, 1.0)

pub const darkgrey = Rgb(169, 169, 169, 1.0)

pub const darkkhaki = Rgb(189, 183, 107, 1.0)

pub const darkmagenta = Rgb(139, 0, 139, 1.0)

pub const darkolivegreen = Rgb(85, 107, 47, 1.0)

pub const darkorange = Rgb(255, 140, 0, 1.0)

pub const darkorchid = Rgb(153, 50, 204, 1.0)

pub const darkred = Rgb(139, 0, 0, 1.0)

pub const darksalmon = Rgb(233, 150, 122, 1.0)

pub const darkseagreen = Rgb(143, 188, 143, 1.0)

pub const darkslateblue = Rgb(72, 61, 139, 1.0)

pub const darkslategray = Rgb(47, 79, 79, 1.0)

pub const darkslategrey = Rgb(47, 79, 79, 1.0)

pub const darkturquoise = Rgb(0, 206, 209, 1.0)

pub const darkviolet = Rgb(148, 0, 211, 1.0)

pub const deeppink = Rgb(255, 20, 147, 1.0)

pub const deepskyblue = Rgb(0, 191, 255, 1.0)

pub const dimgray = Rgb(105, 105, 105, 1.0)

pub const dimgrey = Rgb(105, 105, 105, 1.0)

pub const dodgerblue = Rgb(30, 144, 255, 1.0)

pub const firebrick = Rgb(178, 34, 34, 1.0)

pub const floralwhite = Rgb(255, 250, 240, 1.0)

pub const forestgreen = Rgb(34, 139, 34, 1.0)

pub const fuchsia = Rgb(255, 0, 255, 1.0)

pub const gainsboro = Rgb(220, 220, 220, 1.0)

pub const ghostwhite = Rgb(248, 248, 255, 1.0)

pub const gold = Rgb(255, 215, 0, 1.0)

pub const goldenrod = Rgb(218, 165, 32, 1.0)

pub const gray = Rgb(128, 128, 128, 1.0)

pub const green = Rgb(0, 128, 0, 1.0)

pub const greenyellow = Rgb(173, 255, 47, 1.0)

pub const grey = Rgb(128, 128, 128, 1.0)

pub const honeydew = Rgb(240, 255, 240, 1.0)

pub const hotpink = Rgb(255, 105, 180, 1.0)

pub const indianred = Rgb(205, 92, 92, 1.0)

pub const indigo = Rgb(75, 0, 130, 1.0)

pub const ivory = Rgb(255, 255, 240, 1.0)

pub const khaki = Rgb(240, 230, 140, 1.0)

pub const lavender = Rgb(230, 230, 250, 1.0)

pub const lavenderblush = Rgb(255, 240, 245, 1.0)

pub const lawngreen = Rgb(124, 252, 0, 1.0)

pub const lemonchiffon = Rgb(255, 250, 205, 1.0)

pub const lightblue = Rgb(173, 216, 230, 1.0)

pub const lightcoral = Rgb(240, 128, 128, 1.0)

pub const lightcyan = Rgb(224, 255, 255, 1.0)

pub const lightgoldenrodyellow = Rgb(250, 250, 210, 1.0)

pub const lightgray = Rgb(211, 211, 211, 1.0)

pub const lightgreen = Rgb(144, 238, 144, 1.0)

pub const lightgrey = Rgb(211, 211, 211, 1.0)

pub const lightpink = Rgb(255, 182, 193, 1.0)

pub const lightsalmon = Rgb(255, 160, 122, 1.0)

pub const lightseagreen = Rgb(32, 178, 170, 1.0)

pub const lightskyblue = Rgb(135, 206, 250, 1.0)

pub const lightslategray = Rgb(119, 136, 153, 1.0)

pub const lightslategrey = Rgb(119, 136, 153, 1.0)

pub const lightsteelblue = Rgb(176, 196, 222, 1.0)

pub const lightyellow = Rgb(255, 255, 224, 1.0)

pub const lime = Rgb(0, 255, 0, 1.0)

pub const limegreen = Rgb(50, 205, 50, 1.0)

pub const linen = Rgb(250, 240, 230, 1.0)

pub const magenta = Rgb(255, 0, 255, 1.0)

pub const maroon = Rgb(128, 0, 0, 1.0)

pub const mediumaquamarine = Rgb(102, 205, 170, 1.0)

pub const mediumblue = Rgb(0, 0, 205, 1.0)

pub const mediumorchid = Rgb(186, 85, 211, 1.0)

pub const mediumpurple = Rgb(147, 112, 219, 1.0)

pub const mediumseagreen = Rgb(60, 179, 113, 1.0)

pub const mediumslateblue = Rgb(123, 104, 238, 1.0)

pub const mediumspringgreen = Rgb(0, 250, 154, 1.0)

pub const mediumturquoise = Rgb(72, 209, 204, 1.0)

pub const mediumvioletred = Rgb(199, 21, 133, 1.0)

pub const midnightblue = Rgb(25, 25, 112, 1.0)

pub const mintcream = Rgb(245, 255, 250, 1.0)

pub const mistyrose = Rgb(255, 228, 225, 1.0)

pub const moccasin = Rgb(255, 228, 181, 1.0)

pub const navajowhite = Rgb(255, 222, 173, 1.0)

pub const navy = Rgb(0, 0, 128, 1.0)

pub const oldlace = Rgb(253, 245, 230, 1.0)

pub const olive = Rgb(128, 128, 0, 1.0)

pub const olivedrab = Rgb(107, 142, 35, 1.0)

pub const orange = Rgb(255, 165, 0, 1.0)

pub const orangered = Rgb(255, 69, 0, 1.0)

pub const orchid = Rgb(218, 112, 214, 1.0)

pub const palegoldenrod = Rgb(238, 232, 170, 1.0)

pub const palegreen = Rgb(152, 251, 152, 1.0)

pub const paleturquoise = Rgb(175, 238, 238, 1.0)

pub const palevioletred = Rgb(219, 112, 147, 1.0)

pub const papayawhip = Rgb(255, 239, 213, 1.0)

pub const peachpuff = Rgb(255, 218, 185, 1.0)

pub const peru = Rgb(205, 133, 63, 1.0)

pub const pink = Rgb(255, 192, 203, 1.0)

pub const plum = Rgb(221, 160, 221, 1.0)

pub const powderblue = Rgb(176, 224, 230, 1.0)

pub const purple = Rgb(128, 0, 128, 1.0)

pub const red = Rgb(255, 0, 0, 1.0)

pub const rosybrown = Rgb(188, 143, 143, 1.0)

pub const royalblue = Rgb(65, 105, 225, 1.0)

pub const saddlebrown = Rgb(139, 69, 19, 1.0)

pub const salmon = Rgb(250, 128, 114, 1.0)

pub const sandybrown = Rgb(244, 164, 96, 1.0)

pub const seagreen = Rgb(46, 139, 87, 1.0)

pub const seashell = Rgb(255, 245, 238, 1.0)

pub const sienna = Rgb(160, 82, 45, 1.0)

pub const silver = Rgb(192, 192, 192, 1.0)

pub const skyblue = Rgb(135, 206, 235, 1.0)

pub const slateblue = Rgb(106, 90, 205, 1.0)

pub const slategray = Rgb(112, 128, 144, 1.0)

pub const slategrey = Rgb(112, 128, 144, 1.0)

pub const snow = Rgb(255, 250, 250, 1.0)

pub const springgreen = Rgb(0, 255, 127, 1.0)

pub const steelblue = Rgb(70, 130, 180, 1.0)

pub const tan = Rgb(210, 180, 140, 1.0)

pub const teal = Rgb(0, 128, 128, 1.0)

pub const thistle = Rgb(216, 191, 216, 1.0)

pub const tomato = Rgb(255, 99, 71, 1.0)

pub const turquoise = Rgb(64, 224, 208, 1.0)

pub const violet = Rgb(238, 130, 238, 1.0)

pub const wheat = Rgb(245, 222, 179, 1.0)

pub const white = Rgb(255, 255, 255, 1.0)

pub const whitesmoke = Rgb(245, 245, 245, 1.0)

pub const yellow = Rgb(255, 255, 0, 1.0)

pub const yellowgreen = Rgb(154, 205, 50, 1.0)
