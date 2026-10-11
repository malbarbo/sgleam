import sgleam/fill
import sgleam/image.{right_triangle, to_svg}

pub fn smain() {
  right_triangle(36, 48, fill.black)
  |> to_svg
}
