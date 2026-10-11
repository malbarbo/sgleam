import sgleam/fill
import sgleam/image.{to_svg, triangle_ass}

pub fn smain() {
  triangle_ass(10, 60, 100, fill.seagreen)
  |> to_svg
}
