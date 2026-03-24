import gleam/io
import sgleam/image.{add_wedge, circle, to_svg}
import sgleam/fill
import sgleam/stroke

pub fn main() {
  circle(40, stroke.black)
  |> add_wedge(40, 40, 40, 90, fill.red)
  |> to_svg
  |> io.println
}
