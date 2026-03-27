import gleam/io
import sgleam/fill
import sgleam/font.{Font}
import sgleam/image.{text_font, to_svg}

pub fn main() {
  text_font(
    "Underlined",
    Font(..font.default(), size: 18.0, underline: True),
    fill.green,
  )
  |> to_svg
  |> io.println
}
