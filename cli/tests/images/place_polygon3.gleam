import gleam/io
import sgleam/fill
import sgleam/image.{Point, place_polygon, square, to_svg}

pub fn main() {
  square(50, fill.lightblue)
  |> place_polygon(
    [
      Point(25, -10),
      Point(60, 25),
      Point(25, 60),
      Point(-10, 25),
    ],
    fill.pink,
  )
  |> to_svg
  |> io.println
}
