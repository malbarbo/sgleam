import gleam/io
import sgleam/fill
import sgleam/image.{isosceles_triangle, to_svg}

pub fn main() {
  isosceles_triangle(200, 170, fill.seagreen)
  |> to_svg
  |> io.println
}
