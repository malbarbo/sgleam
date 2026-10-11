import sgleam/fill
import sgleam/image.{ellipse, overlay, rectangle, to_svg}

pub fn smain() {
  rectangle(30, 60, fill.orange)
  |> overlay(ellipse(60, 30, fill.purple))
  |> to_svg
}
