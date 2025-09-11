import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, overlay_align, rectangle, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  rectangle(30, 60, fill.orange)
  |> overlay_align(xplace.Left, yplace.Middle, _, ellipse(60, 30, fill.purple))
  |> to_svg
  |> io.println
}
