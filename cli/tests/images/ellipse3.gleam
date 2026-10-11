import sgleam/image.{beside, ellipse, to_svg}
import sgleam/stroke

pub fn smain() {
  beside(ellipse(0, 20, stroke.black), ellipse(20, 0, stroke.black))
  |> to_svg
}
