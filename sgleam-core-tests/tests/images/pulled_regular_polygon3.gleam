import gleam/io
import sgleam/image.{pulled_regular_polygon, to_svg}
import sgleam/fill

pub fn main() {
  pulled_regular_polygon(50, 4, 0.0, 0.0, fill.orange)
  |> to_svg
  |> io.println
}
