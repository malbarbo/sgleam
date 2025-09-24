import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, rectangle, to_svg, underlay_align}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  rectangle(30, 60, fill.orange)
  |> underlay_align(xplace.Left, yplace.Middle, ellipse(60, 30, fill.purple))
  |> to_svg
  |> io.println
}
