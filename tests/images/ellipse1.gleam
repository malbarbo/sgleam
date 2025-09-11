import gleam/io
import sgleam/image.{ellipse, to_svg}
import sgleam/stroke

pub fn main() {
  ellipse(60, 30, stroke.black)
  |> to_svg
  |> io.println
}
