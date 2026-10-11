import sgleam/fill
import sgleam/image.{ellipse, frame, to_svg}

pub fn smain() {
  ellipse(40, 40, fill.gray)
  |> frame
  |> to_svg
}
