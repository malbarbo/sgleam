import sgleam/fill
import sgleam/image.{regular_polygon, to_svg}

pub fn smain() {
  regular_polygon(40, 4, fill.blue)
  |> to_svg
}
