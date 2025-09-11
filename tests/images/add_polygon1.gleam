import gleam/io
import sgleam/fill
import sgleam/image.{Point, add_polygon, square, to_svg}

pub fn main() {
  square(65, fill.lightblue)
  |> add_polygon(
    [Point(30, -20), Point(50, 50), Point(-20, 30)],
    fill.forestgreen,
  )
  |> to_svg
  |> io.println
}
