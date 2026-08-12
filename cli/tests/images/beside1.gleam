import gleam/io
import sgleam/fill
import sgleam/image.{beside, ellipse, to_svg}

pub fn main() {
  ellipse(20, 70, fill.lightgray)
  |> beside(ellipse(20, 50, fill.darkgray))
  |> beside(ellipse(20, 30, fill.dimgray))
  |> beside(ellipse(20, 10, fill.black))
  |> to_svg
  |> io.println
}
