import sgleam/fill
import sgleam/image.{Point, polygon, rectangle, to_svg, underlay}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  rectangle(80, 80, fill.mediumseagreen)
  |> underlay(polygon(
    [Point(0, 0), Point(50, 0), Point(0, 50), Point(50, 50)],
    style.join([
      stroke.darkslategray,
      stroke.width(10),
      stroke.linecap_square,
      stroke.linejoin_miter,
    ]),
  ))
  |> to_svg
}
