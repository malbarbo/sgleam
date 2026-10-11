import sgleam/fill
import sgleam/image.{circle, to_svg, underlay_offset}

pub fn smain() {
  circle(40, fill.red)
  |> underlay_offset(10, 10, circle(40, fill.blue))
  |> to_svg
}
