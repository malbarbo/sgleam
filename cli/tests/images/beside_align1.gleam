import gleam/io
import sgleam/fill
import sgleam/image.{beside_align, ellipse, to_svg}
import sgleam/yplace

pub fn main() {
  ellipse(20, 70, fill.lightsteelblue)
  |> beside_align(yplace.Bottom, ellipse(20, 50, fill.mediumslateblue))
  |> beside_align(yplace.Bottom, ellipse(20, 30, fill.slateblue))
  |> beside_align(yplace.Bottom, ellipse(20, 10, fill.navy))
  |> to_svg
  |> io.println
}
