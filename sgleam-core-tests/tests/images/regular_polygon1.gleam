import gleam/io
import sgleam/image.{regular_polygon, to_svg}
import sgleam/stroke

pub fn main() {
  regular_polygon(50, 3, stroke.red)
  |> to_svg
  |> io.println
}
