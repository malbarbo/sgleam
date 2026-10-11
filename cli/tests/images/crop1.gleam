import sgleam/fill
import sgleam/image.{circle, crop, to_svg}

pub fn smain() {
  circle(40, fill.chocolate)
  |> crop(0, 0, 40, 40)
  |> to_svg
}
