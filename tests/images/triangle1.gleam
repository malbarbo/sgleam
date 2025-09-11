import gleam/io
import sgleam/fill
import sgleam/image.{to_svg, triangle}

pub fn main() {
  triangle(40, fill.tan)
  |> to_svg
  |> io.println
}
