import sgleam/image.{to_svg, triangle_sss}
import sgleam/stroke

pub fn smain() {
  triangle_sss(40, 60, 80, stroke.black)
  |> to_svg
}
