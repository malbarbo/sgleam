import gleam/io
import sgleam/image.{beside, ellipse, to_svg}
import sgleam/stroke

pub fn main() {
  beside(ellipse(0, 20, stroke.black), ellipse(20, 0, stroke.black))
  |> to_svg
  |> io.println
}
