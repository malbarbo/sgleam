import gleam/io
import sgleam/image.{line, to_svg}
import sgleam/stroke

pub fn main() {
  line(-30, 20, stroke.red)
  |> to_svg
  |> io.println
}
