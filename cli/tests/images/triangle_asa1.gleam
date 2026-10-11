import sgleam/fill
import sgleam/image.{to_svg, triangle_asa}

pub fn smain() {
  triangle_asa(10, 200, 40, fill.seagreen)
  |> to_svg
}
