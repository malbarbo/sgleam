import gleam/io
import sgleam/fill
import sgleam/image.{beside, ellipse, scale, to_svg}

pub fn main() {
  ellipse(20, 30, fill.blue)
  |> scale(2)
  |> beside(ellipse(40, 60, fill.blue))
  |> to_svg
  |> io.println
}
