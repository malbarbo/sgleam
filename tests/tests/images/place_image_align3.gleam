import gleam/io
import sgleam/fill
import sgleam/image.{
  beside, circle, combine, place_image_align, rectangle, to_svg,
}
import sgleam/stroke
import sgleam/xplace
import sgleam/yplace

import stroke

pub fn main() {
  [
    rectangle(32, 32, stroke.black)
      |> place_image_align(0, 0, xplace.Center, yplace.Middle, circle(8, fill.tomato)),
    rectangle(32, 32, stroke.black)
      |> place_image_align(8, 8, xplace.Center, yplace.Middle, circle(8, fill.tomato)),
    rectangle(32, 32, stroke.black)
      |> place_image_align(16, 16, xplace.Center, yplace.Middle, circle(8, fill.tomato)),
    rectangle(32, 32, stroke.black)
      |> place_image_align(24, 24, xplace.Center, yplace.Middle, circle(8, fill.tomato)),
    rectangle(32, 32, stroke.black)
      |> place_image_align(32, 32, xplace.Center, yplace.Middle, circle(8, fill.tomato)),
  ]
  |> combine(beside)
  |> to_svg
  |> io.println
}
