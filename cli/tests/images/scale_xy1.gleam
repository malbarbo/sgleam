import gleam/io
import sgleam/fill
import sgleam/image.{beside, ellipse, scale_xy, to_svg}

pub fn main() {
  ellipse(20, 30, fill.blue)
  |> scale_xy(3, 2)
  |> beside(ellipse(60, 60, fill.blue))
  |> to_svg
  |> io.println
}
