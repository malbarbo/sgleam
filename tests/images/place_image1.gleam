import gleam/io
import sgleam/fill
import sgleam/image.{place_image, rectangle, to_svg, triangle}

pub fn main() {
  rectangle(48, 48, fill.lightgray)
  |> place_image(24, 24, triangle(32, fill.red))
  |> to_svg
  |> io.println
}
