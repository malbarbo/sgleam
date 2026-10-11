import sgleam/fill
import sgleam/image.{to_svg, wedge}

pub fn smain() {
  wedge(40, 360, fill.blue)
  |> to_svg
}
