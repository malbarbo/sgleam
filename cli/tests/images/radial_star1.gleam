import sgleam/fill
import sgleam/image.{radial_star, to_svg}

pub fn smain() {
  radial_star(8, 8, 64, fill.darkslategray)
  |> to_svg
}
