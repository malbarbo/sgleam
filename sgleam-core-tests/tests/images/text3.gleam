import gleam/io
import sgleam/fill
import sgleam/font.{Font, Italic}
import sgleam/image.{text_font, to_svg}

pub fn main() {
  text_font(
    "Italic text",
    Font(..font.default(), size: 20.0, font_style: Italic),
    fill.blue,
  )
  |> to_svg
  |> io.println
}
