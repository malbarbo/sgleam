import gleam/io
import sgleam/fill
import sgleam/image.{Point, polygon, to_svg}

pub fn main() {
  polygon(
    [Point(0, 0), Point(-10, 20), Point(60, 0), Point(-10, -20)],
    fill.burlywood,
  )
  |> to_svg
  |> io.println
}
