import sgleam/image.{star_polygon, to_svg}
import sgleam/stroke

pub fn smain() {
  star_polygon(40, 7, 3, stroke.darkred)
  |> to_svg
}
