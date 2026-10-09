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
