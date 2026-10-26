import sgleam/fill
import sgleam/image.{place_image, rectangle, star, to_svg, triangle}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  let miter = style.join([stroke.red, stroke.width(4), stroke.linejoin_miter])
  rectangle(100, 80, fill.lightgray)
  |> place_image(95, 40, star(30, miter))
  |> place_image(5, 40, triangle(30, miter))
  |> to_svg
}
