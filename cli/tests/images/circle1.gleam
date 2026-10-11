import sgleam/image.{circle, to_svg}
import sgleam/stroke

pub fn smain() {
  circle(30, stroke.red)
  |> to_svg
}
