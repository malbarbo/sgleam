import sgleam/fill
import sgleam/image.{to_svg, triangle_sas}

pub fn smain() {
  triangle_sas(60, 10, 100, fill.seagreen)
  |> to_svg
}
