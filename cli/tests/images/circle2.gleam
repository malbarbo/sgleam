import gleam/io
import sgleam/fill
import sgleam/image.{circle, to_svg}

pub fn main() {
  circle(20, fill.blue)
  |> to_svg
  |> io.println
}
