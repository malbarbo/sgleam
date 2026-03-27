import gleam/io
import sgleam/fill
import sgleam/image.{place_image_align, rectangle, rotate, to_svg, triangle}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  rectangle(64, 64, fill.palegoldenrod)
  |> place_image_align(
    0,
    0,
    xplace.Left,
    yplace.Top,
    triangle(48, fill.yellowgreen) |> rotate(180),
  )
  |> to_svg
  |> io.println
}
