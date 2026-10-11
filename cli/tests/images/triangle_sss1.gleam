import sgleam/fill
import sgleam/image.{to_svg, triangle_sss}

pub fn smain() {
  triangle_sss(40, 60, 80, fill.seagreen)
  |> to_svg
}
