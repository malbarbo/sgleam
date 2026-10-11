import sgleam/fill
import sgleam/image.{crop, ellipse, to_svg}

pub fn smain() {
  ellipse(80, 120, fill.dodgerblue)
  |> crop(40, 60, 40, 60)
  |> to_svg
}
