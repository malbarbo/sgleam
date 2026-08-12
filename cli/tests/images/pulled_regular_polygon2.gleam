import gleam/io
import sgleam/image.{pulled_regular_polygon, to_svg}
import sgleam/stroke

pub fn main() {
  pulled_regular_polygon(50, 6, 0.5, 45.0, stroke.red)
  |> to_svg
  |> io.println
}
