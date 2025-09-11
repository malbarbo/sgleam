import gleam/io
import sgleam/image.{ellipse, place_line, to_svg}
import sgleam/stroke

pub fn main() {
  ellipse(40, 40, stroke.maroon)
  |> place_line(0, 40, 40, 0, stroke.maroon)
  |> to_svg
  |> io.println
}
