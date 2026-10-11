import sgleam/fill
import sgleam/image.{regular_polygon, to_svg}

pub fn smain() {
  regular_polygon(20, 8, fill.red)
  |> to_svg
}
