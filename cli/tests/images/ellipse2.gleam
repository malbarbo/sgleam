import sgleam/fill
import sgleam/image.{ellipse, to_svg}

pub fn smain() {
  ellipse(30, 60, fill.blue)
  |> to_svg
}
