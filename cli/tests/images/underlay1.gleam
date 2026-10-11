import sgleam/fill
import sgleam/image.{ellipse, rectangle, to_svg, underlay}

pub fn smain() {
  rectangle(30, 60, fill.orange)
  |> underlay(ellipse(60, 30, fill.purple))
  |> to_svg
}
