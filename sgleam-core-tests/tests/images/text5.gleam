import gleam/io
import sgleam/fill
import sgleam/font.{Bold, Font, Italic}
import sgleam/image.{text_font, to_svg}

pub fn main() {
  text_font(
    "Bold Italic Underline",
    Font(
      ..font.default(),
      size: 24.0,
      font_style: Italic,
      font_weight: Bold,
      underline: True,
    ),
    fill.black,
  )
  |> to_svg
  |> io.println
}
