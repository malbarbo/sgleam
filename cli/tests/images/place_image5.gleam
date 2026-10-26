import sgleam/fill
import sgleam/image.{circle, place_image, rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  let thick = style.join([stroke.red, stroke.width(9)])
  rectangle(100, 80, fill.lightgray)
  |> place_image(0, 0, rectangle(30, 30, thick))
  |> place_image(100, 80, rectangle(30, 30, thick))
  |> place_image(50, 0, circle(10, thick))
  |> to_svg
}
