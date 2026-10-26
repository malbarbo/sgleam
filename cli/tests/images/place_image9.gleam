import sgleam/fill
import sgleam/image.{circle, place_image, rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  rectangle(100, 80, fill.lightgray)
  |> place_image(50, 40, circle(90, style.join([stroke.red, stroke.width(10)])))
  |> place_image(20, 20, rectangle(10, 10, fill.black))
  |> to_svg
}
