import sgleam/image.{radial_star, to_svg}
import sgleam/stroke

pub fn smain() {
  radial_star(32, 30, 40, stroke.black)
  |> to_svg
}
