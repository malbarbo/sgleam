import gleam/io
import sgleam/fill
import sgleam/image.{Point, polygon, to_svg}

pub fn main() {
  polygon(
    [
      Point(0, 0),
      Point(0, 40),
      Point(20, 40),
      Point(20, 60),
      Point(40, 60),
      Point(40, 20),
      Point(20, 20),
      Point(20, 0),
    ],
    fill.plum,
  )
  |> to_svg
  |> io.println
}
