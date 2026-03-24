import gleam/io
import sgleam/fill
import sgleam/image.{place_line, rectangle, to_svg}
import sgleam/stroke

pub fn main() {
  rectangle(40, 40, fill.lightgray)
  |> place_line(-10, 50, 50, -10, stroke.maroon)
  |> to_svg
  |> io.println
}
