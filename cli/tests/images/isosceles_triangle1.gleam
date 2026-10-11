import sgleam/fill
import sgleam/image.{isosceles_triangle, to_svg}

pub fn smain() {
  isosceles_triangle(200, 170, fill.seagreen)
  |> to_svg
}
