import gleam/io
import sgleam/fill
import sgleam/image.{square, to_svg, underlay_align}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  square(50, fill.seagreen)
  |> underlay_align(xplace.Right, yplace.Top, square(40, fill.silver))
  |> underlay_align(xplace.Right, yplace.Top, square(30, fill.seagreen))
  |> underlay_align(xplace.Right, yplace.Top, square(20, fill.silver))
  |> to_svg
  |> io.println
}
