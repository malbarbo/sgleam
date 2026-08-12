import gleam/io
import sgleam/fill
import sgleam/image.{Point, circle, place_images, rectangle, to_svg}

pub fn main() {
  rectangle(24, 24, fill.goldenrod)
  |> place_images([Point(18, 20), Point(0, 6), Point(14, 2)], [
    circle(4, fill.white),
    circle(4, fill.white),
    circle(4, fill.white),
  ])
  |> to_svg
  |> io.println
}
