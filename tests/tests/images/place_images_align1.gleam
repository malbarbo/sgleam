import gleam/io
import sgleam/fill
import sgleam/image.{Point, place_images_align, rectangle, to_svg, triangle}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  rectangle(64, 64, fill.goldenrod)
  |> place_images_align(
    [Point(64, 64), Point(64, 48), Point(64, 32)],
    xplace.Right,
    yplace.Bottom,
    [
      triangle(48, fill.yellowgreen),
      triangle(48, fill.yellowgreen),
      triangle(48, fill.yellowgreen),
    ],
  )
  |> to_svg
  |> io.println
}
