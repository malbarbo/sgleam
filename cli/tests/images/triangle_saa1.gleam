import sgleam/fill
import sgleam/image.{to_svg, triangle_saa}

pub fn smain() {
  triangle_saa(100, 10, 40, fill.seagreen)
  |> to_svg
}
