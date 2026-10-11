import sgleam/fill
import sgleam/image.{beside, ellipse, scale, to_svg}

pub fn smain() {
  ellipse(20, 30, fill.blue)
  |> scale(2)
  |> beside(ellipse(40, 60, fill.blue))
  |> to_svg
}
