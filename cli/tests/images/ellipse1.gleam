import sgleam/image.{ellipse, to_svg}
import sgleam/stroke

pub fn smain() {
  ellipse(60, 30, stroke.black)
  |> to_svg
}
