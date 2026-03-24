import gleam/io
import sgleam/image.{radial_star, to_svg}
import sgleam/stroke

pub fn main() {
  radial_star(32, 30, 40, stroke.black)
  |> to_svg
  |> io.println
}
