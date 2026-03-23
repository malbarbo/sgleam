import gleam/io
import sgleam/image.{add_curve, rectangle, to_svg}
import sgleam/stroke

pub fn main() {
  rectangle(100, 100, stroke.black)
  |> add_curve(50, 10, 270, 0.5, 50, 90, 90, 0.5, stroke.red)
  |> to_svg
  |> io.println
}
