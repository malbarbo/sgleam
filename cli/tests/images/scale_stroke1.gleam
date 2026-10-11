import sgleam/image.{beside, rectangle, scale, scale_xy, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  let thick = rectangle(20, 20, style.join([stroke.black, stroke.width(2)]))
  let thin = rectangle(20, 20, stroke.black)
  beside(
    beside(thick, scale(thick, 3)),
    beside(scale_xy(thick, 4, 1), scale(thin, 4)),
  )
  |> to_svg
}
