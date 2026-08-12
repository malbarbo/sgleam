import gleam/io
import sgleam/image.{add_curve, rectangle, to_svg}
import sgleam/stroke

pub fn main() {
  rectangle(100, 100, stroke.black)
  |> add_curve(20, 20, 0, 0.333, 80, 80, 0, 0.333, stroke.red)
  |> to_svg
  |> io.println
}
