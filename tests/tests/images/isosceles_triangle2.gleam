import gleam/io
import sgleam/fill
import sgleam/image.{isosceles_triangle, to_svg}

pub fn main() {
  isosceles_triangle(60, 30, fill.aquamarine)
  |> to_svg
  |> io.println
}
