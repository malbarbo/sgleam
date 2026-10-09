import gleam/io
import sgleam/fill
import sgleam/image.{beside, ellipse, rotate, scale_xy, to_svg}

pub fn main() {
  let e = rotate(ellipse(40, 10, fill.blue), 45)
  beside(beside(e, scale_xy(e, 2, 1)), scale_xy(e, 1, 3))
  |> to_svg
  |> io.println
}
