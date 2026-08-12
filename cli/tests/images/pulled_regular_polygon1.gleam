import gleam/io
import sgleam/image.{pulled_regular_polygon, to_svg}
import sgleam/fill

pub fn main() {
  pulled_regular_polygon(50, 5, 0.333, 30.0, fill.green)
  |> to_svg
  |> io.println
}
