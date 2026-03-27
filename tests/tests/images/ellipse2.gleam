import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, to_svg}

pub fn main() {
  ellipse(30, 60, fill.blue)
  |> to_svg
  |> io.println
}
