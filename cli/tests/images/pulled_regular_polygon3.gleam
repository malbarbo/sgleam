import sgleam/image.{pulled_regular_polygon, to_svg}
import sgleam/fill

pub fn smain() {
  pulled_regular_polygon(50, 4, 0.0, 0.0, fill.orange)
  |> to_svg
}
