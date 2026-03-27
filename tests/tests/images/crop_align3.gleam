import gleam/io
import sgleam/fill
import sgleam/image.{circle, crop_align, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  circle(25, fill.mediumslateblue)
  |> crop_align(xplace.Center, yplace.Middle, 50, 30)
  |> to_svg
  |> io.println
}
