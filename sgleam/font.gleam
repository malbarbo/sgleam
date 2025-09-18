pub type Font {
  Font(family: String, size: Float)
}

pub fn default() -> Font {
  Font("sans-serif", 14.0)
}
