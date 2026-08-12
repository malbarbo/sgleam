import gleam/io
import sgleam/fill
import sgleam/image.{isosceles_triangle, to_svg}

pub fn main() {
  isosceles_triangle(60, 330, fill.lightseagreen)
  |> to_svg
  |> io.println
}
