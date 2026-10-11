import sgleam/fill
import sgleam/image.{to_svg, triangle_aas}

pub fn smain() {
  triangle_aas(10, 40, 200, fill.seagreen)
  |> to_svg
}
