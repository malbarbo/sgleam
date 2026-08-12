import gleam/io
import sgleam/fill
import sgleam/font.{Bold, Font}
import sgleam/image.{text_font, to_svg}

pub fn main() {
  text_font(
    "Bold text",
    Font(..font.default(), size: 20.0, font_weight: Bold),
    fill.red,
  )
  |> to_svg
  |> io.println
}
