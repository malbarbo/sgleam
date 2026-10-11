import sgleam/fill
import sgleam/image.{to_svg, triangle_ssa}

pub fn smain() {
  triangle_ssa(60, 100, 10, fill.seagreen)
  |> to_svg
}
