import sgleam/fill
import sgleam/image.{circle, crop, place_image, rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  circle(60, fill.teal)
  |> crop(20, 20, 80, 60)
  |> place_image(0, 0, rectangle(30, 30, style.join([stroke.red, stroke.width(8)])))
  |> place_image(80, 60, circle(15, fill.black))
  |> to_svg
}
