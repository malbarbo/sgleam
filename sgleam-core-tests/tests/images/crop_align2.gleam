import gleam/io
import sgleam/fill
import sgleam/image.{crop_align, ellipse, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  ellipse(80, 120, fill.dodgerblue)
  |> crop_align(xplace.Right, yplace.Bottom, 40, 60)
  |> to_svg
  |> io.println
}
