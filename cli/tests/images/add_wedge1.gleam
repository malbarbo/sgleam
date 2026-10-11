import sgleam/fill
import sgleam/image.{add_wedge, circle, to_svg}
import sgleam/stroke

pub fn smain() {
  circle(40, stroke.black)
  |> add_wedge(40, 40, 40, 90, fill.red)
  |> to_svg
}
