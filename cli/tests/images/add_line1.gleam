import sgleam/image.{add_line, ellipse, to_svg}
import sgleam/stroke

pub fn smain() {
  ellipse(40, 40, stroke.maroon)
  |> add_line(0, 40, 40, 0, stroke.maroon)
  |> to_svg
}
