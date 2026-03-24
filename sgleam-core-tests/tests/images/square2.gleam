import gleam/io
import sgleam/image.{square, to_svg}
import sgleam/stroke

pub fn main() {
  square(50, stroke.darkmagenta)
  |> to_svg
  |> io.println
}
