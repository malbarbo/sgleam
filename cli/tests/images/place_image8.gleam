import sgleam/fill
import sgleam/image.{place_image, rectangle, text, to_svg}

pub fn smain() {
  rectangle(100, 80, fill.lightgray)
  |> place_image(90, 40, text("Overhang", 30, fill.black))
  |> place_image(10, 70, text("gjpq", 40, fill.green))
  |> to_svg
}
