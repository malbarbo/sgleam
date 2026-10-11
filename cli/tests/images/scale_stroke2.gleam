import sgleam/image.{beside, rectangle, scale, scale_xy, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  let dashed =
    rectangle(
      20,
      20,
      style.join([stroke.black, stroke.width(2), stroke.dash_array([4, 2])]),
    )
  beside(beside(dashed, scale(dashed, 3)), scale_xy(dashed, 4, 1))
  |> to_svg
}
