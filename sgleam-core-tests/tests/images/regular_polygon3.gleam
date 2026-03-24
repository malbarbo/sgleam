import gleam/io
import sgleam/fill
import sgleam/image.{regular_polygon, to_svg}

pub fn main() {
  regular_polygon(20, 8, fill.red)
  |> to_svg
  |> io.println
}
