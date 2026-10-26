import sgleam/fill
import sgleam/image.{circle, place_image, rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  let thick = style.join([stroke.red, stroke.width(6)])
  let inner =
    rectangle(40, 40, fill.yellow)
    |> place_image(0, 20, circle(15, thick))
    |> place_image(40, 20, circle(15, thick))
  rectangle(100, 80, fill.lightgray)
  |> place_image(50, 40, inner)
  |> place_image(100, 0, circle(15, thick))
  |> to_svg
}
