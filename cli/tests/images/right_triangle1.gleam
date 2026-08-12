import gleam/io
import sgleam/fill
import sgleam/image.{right_triangle, to_svg}

pub fn main() {
  right_triangle(36, 48, fill.black)
  |> to_svg
  |> io.println
}
