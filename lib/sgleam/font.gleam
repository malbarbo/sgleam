import gleam/float

pub type Font {
  Font(
    family: String,
    size: Float,
    font_style: FontStyle,
    font_weight: FontWeight,
    underline: Bool,
  )
}

pub type FontStyle {
  Normal
  Italic
  Slant
}

pub type FontWeight {
  Light
  Regular
  Bold
}

pub fn default() -> Font {
  Font("sans-serif", 14.0, Normal, Regular, False)
}

pub fn font_style_to_svg(s: FontStyle) -> String {
  case s {
    Normal -> "normal"
    Italic -> "italic"
    Slant -> "oblique"
  }
}

pub fn font_weight_to_svg(w: FontWeight) -> String {
  case w {
    Light -> "lighter"
    Regular -> "normal"
    Bold -> "bold"
  }
}

/// CSS font shorthand: "italic bold 24px sans-serif"
pub fn to_css(font: Font) -> String {
  let style = case font.font_style {
    Normal -> ""
    Italic -> "italic "
    Slant -> "oblique "
  }
  let weight = case font.font_weight {
    Regular -> ""
    Bold -> "bold "
    Light -> "lighter "
  }
  style <> weight <> float.to_string(font.size) <> "px " <> font.family
}
