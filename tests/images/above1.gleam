import gleam/io
import sgleam/fill
import sgleam/image.{above, ellipse, to_svg}

pub fn main() {
  ellipse(70, 20, fill.lightgray)
  |> above(ellipse(50, 20, fill.darkgray))
  |> above(ellipse(30, 20, fill.dimgray))
  |> above(ellipse(10, 20, fill.black))
  |> to_svg
  |> io.println
}
