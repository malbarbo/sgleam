import sgleam/fill
import sgleam/image.{place_image, rectangle, to_svg}
import sgleam/stroke

pub fn smain() {
  rectangle(100, 80, fill.lightgray)
  |> place_image(50, 40, rectangle(100, 80, stroke.black))
  |> place_image(50, 40, rectangle(98, 78, stroke.green))
  |> to_svg
}
