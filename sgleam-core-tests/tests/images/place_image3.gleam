import gleam/io
import sgleam/fill
import sgleam/image.{circle, place_image, rectangle, to_svg}

pub fn main() {
  rectangle(24, 24, fill.goldenrod)
  |> place_image(8, 14, circle(4, fill.white))
  |> place_image(14, 2, circle(4, fill.white))
  |> place_image(0, 6, circle(4, fill.white))
  |> place_image(18, 20, circle(4, fill.white))
  |> to_svg
  |> io.println
}
