import sgleam/fill
import sgleam/image.{isosceles_triangle, to_svg}

pub fn smain() {
  isosceles_triangle(60, 30, fill.aquamarine)
  |> to_svg
}
