import gleam/io
import sgleam/image.{rectangle, rotate, to_svg}
import sgleam/stroke

pub fn main() {
  rectangle(50, 50, stroke.black)
  |> rotate(5)
  |> to_svg
  |> io.println
}
