import sgleam/fill
import sgleam/image.{star_polygon, to_svg}

pub fn smain() {
  star_polygon(20, 10, 3, fill.cornflowerblue)
  |> to_svg
}
