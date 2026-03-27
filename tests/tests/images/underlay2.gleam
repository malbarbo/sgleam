import gleam/io
import sgleam/fill
import sgleam/image.{combine, ellipse, to_svg, underlay}

pub fn main() {
  [
    ellipse(10, 60, fill.red),
    ellipse(20, 50, fill.black),
    ellipse(30, 40, fill.red),
    ellipse(40, 30, fill.black),
    ellipse(50, 20, fill.red),
    ellipse(60, 10, fill.black),
  ]
  |> combine(underlay)
  |> to_svg
  |> io.println
}
