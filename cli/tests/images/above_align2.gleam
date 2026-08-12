import gleam/io
import sgleam/fill
import sgleam/image.{above_align, ellipse, to_svg}
import sgleam/xplace

pub fn main() {
  ellipse(70, 20, fill.gold)
  |> above_align(xplace.Left, ellipse(50, 20, fill.goldenrod))
  |> above_align(xplace.Left, ellipse(30, 20, fill.darkgoldenrod))
  |> above_align(xplace.Left, ellipse(10, 20, fill.sienna))
  |> to_svg
  |> io.println
}
