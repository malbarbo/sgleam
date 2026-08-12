import gleam/io
import sgleam/image.{add_curve, rectangle, to_svg}
import sgleam/stroke

pub fn main() {
  rectangle(100, 100, stroke.black)
  |> add_curve(20, 50, 0, 0.0, 80, 50, 0, 0.0, stroke.red)
  |> to_svg
  |> io.println
}
