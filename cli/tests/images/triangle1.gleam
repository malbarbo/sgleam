import sgleam/fill
import sgleam/image.{to_svg, triangle}

pub fn smain() {
  triangle(40, fill.tan)
  |> to_svg
}
