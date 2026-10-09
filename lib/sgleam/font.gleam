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
  style <> weight <> fs(font.size) <> "px " <> font.family
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "float_to_string_6")
fn fs(v: Float) -> String
