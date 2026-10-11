import sgleam/fill
import sgleam/image.{pulled_regular_polygon, to_svg}

pub fn smain() {
  pulled_regular_polygon(50, 5, 0.333, 30.0, fill.green)
  |> to_svg
}
