import gleam/io
import sgleam/fill
import sgleam/image.{Point, add_polygon, square, to_svg}
import sgleam/stroke

pub fn main() {
  square(180, fill.yellow)
  |> add_polygon(
    [
      Point(109, 160),
      Point(26, 148),
      Point(46, 36),
      Point(93, 44),
      Point(89, 68),
      Point(122, 72),
    ],
    stroke.darkblue,
  )
  |> to_svg
  |> io.println
}
