import gleam/io
import sgleam/image.{add_solid_curve, rectangle, to_svg}
import sgleam/fill
import sgleam/stroke

pub fn main() {
  rectangle(100, 100, stroke.black)
  |> add_solid_curve(20, 20, 0, 0.333, 80, 80, 0, 0.333, fill.red)
  |> to_svg
  |> io.println
}
