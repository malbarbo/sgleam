import gleam/io
import sgleam/image.{circle, to_svg}
import sgleam/stroke

pub fn main() {
  circle(30, stroke.red)
  |> to_svg
  |> io.println
}
