import gleam/io
import sgleam/fill
import sgleam/image.{above_align, ellipse, to_svg}
import sgleam/xplace

pub fn main() {
  ellipse(70, 20, fill.yellowgreen)
  |> above_align(xplace.Right, _, ellipse(50, 20, fill.olivedrab))
  |> above_align(xplace.Right, _, ellipse(30, 20, fill.darkolivegreen))
  |> above_align(xplace.Right, _, ellipse(10, 20, fill.darkgreen))
  |> to_svg
  |> io.println
}
