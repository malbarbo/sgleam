import gleam/io
import sgleam/image.{rectangle, to_svg}
import sgleam/stroke

pub fn main() {
  rectangle(40, 20, stroke.black)
  |> to_svg
  |> io.println
}
