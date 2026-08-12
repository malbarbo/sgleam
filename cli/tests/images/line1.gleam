import gleam/io
import sgleam/image.{line, to_svg}
import sgleam/stroke

pub fn main() {
  line(30, 30, stroke.black)
  |> to_svg
  |> io.println
}
