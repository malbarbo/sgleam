import sgleam/fill
import sgleam/image.{circle, to_svg}

pub fn smain() {
  circle(20, fill.blue)
  |> to_svg
}
