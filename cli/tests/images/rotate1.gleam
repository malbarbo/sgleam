import sgleam/fill
import sgleam/image.{ellipse, rotate, to_svg}

pub fn smain() {
  ellipse(60, 20, fill.olivedrab)
  |> rotate(45)
  |> to_svg
}
