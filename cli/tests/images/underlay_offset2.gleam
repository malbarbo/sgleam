import sgleam/fill
import sgleam/image.{circle, to_svg, underlay_offset}

pub fn smain() {
  circle(40, fill.gray)
  |> underlay_offset(
    0,
    -10,
    circle(10, fill.navy)
      |> underlay_offset(-30, 0, circle(10, fill.navy)),
  )
  |> to_svg
}
