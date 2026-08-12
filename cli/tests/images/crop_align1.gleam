import gleam/io
import sgleam/fill
import sgleam/image.{circle, crop_align, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  circle(40, fill.chocolate)
  |> crop_align(xplace.Left, yplace.Top, 40, 40)
  |> to_svg
  |> io.println
}
