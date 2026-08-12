import gleam/io
import sgleam/image.{empty_scene, place_curve, to_svg}
import sgleam/stroke

pub fn main() {
  empty_scene(100, 100)
  |> place_curve(10, 50, 90, 0.5, 90, 50, 90, 0.5, stroke.blue)
  |> to_svg
  |> io.println
}
