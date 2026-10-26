import sgleam/fill
import sgleam/image.{place_image, rectangle, rotate, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  let thick = style.join([stroke.red, stroke.width(8)])
  rectangle(100, 80, fill.lightgray)
  |> place_image(0, 0, rectangle(30, 30, thick))
  |> rotate(30)
  |> place_image(0, 0, rectangle(30, 30, thick))
  |> to_svg
}
