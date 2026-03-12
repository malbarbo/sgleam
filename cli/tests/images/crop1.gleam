import gleam/io
import sgleam/fill
import sgleam/image.{circle, crop, to_svg}

pub fn main() {
  circle(40, fill.chocolate)
  |> crop(0, 0, 40, 40)
  |> to_svg
  |> io.println
}
