import gleam/io
import sgleam/fill
import sgleam/image.{beside_align, ellipse, to_svg}
import sgleam/yplace

pub fn main() {
  ellipse(20, 70, fill.mediumorchid)
  |> beside_align(yplace.Top, ellipse(20, 50, fill.darkorchid))
  |> beside_align(yplace.Top, ellipse(20, 30, fill.purple))
  |> beside_align(yplace.Top, ellipse(20, 10, fill.indigo))
  |> to_svg
  |> io.println
}
