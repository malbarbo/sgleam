import gleam/io
import sgleam/fill
import sgleam/image.{place_image_align, rectangle, to_svg, triangle}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  rectangle(64, 64, fill.palegoldenrod)
  |> place_image_align(
    64,
    64,
    xplace.Right,
    yplace.Bottom,
    triangle(48, fill.yellowgreen),
  )
  |> to_svg
  |> io.println
}
