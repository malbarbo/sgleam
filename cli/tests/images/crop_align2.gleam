import sgleam/fill
import sgleam/image.{crop_align, ellipse, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn smain() {
  ellipse(80, 120, fill.dodgerblue)
  |> crop_align(xplace.Right, yplace.Bottom, 40, 60)
  |> to_svg
}
