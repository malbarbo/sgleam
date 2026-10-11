import sgleam/fill
import sgleam/image.{star_polygon, to_svg}

pub fn smain() {
  star_polygon(40, 5, 2, fill.seagreen)
  |> to_svg
}
