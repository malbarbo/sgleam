import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, overlay, to_svg}

pub fn main() {
  ellipse(10, 10, fill.red)
  |> overlay(ellipse(20, 20, fill.black))
  |> overlay(ellipse(30, 30, fill.red))
  |> overlay(ellipse(40, 40, fill.black))
  |> overlay(ellipse(50, 50, fill.red))
  |> overlay(ellipse(60, 60, fill.black))
  |> to_svg
  |> io.println
}
