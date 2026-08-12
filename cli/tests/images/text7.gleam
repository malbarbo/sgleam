import gleam/io
import sgleam/fill
import sgleam/font.{Font, Light}
import sgleam/image.{text_font, to_svg}

pub fn main() {
  text_font(
    "Light mono",
    Font("monospace", 16.0, font.Normal, Light, False),
    fill.darkgray,
  )
  |> to_svg
  |> io.println
}
