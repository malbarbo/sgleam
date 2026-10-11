import sgleam/fill
import sgleam/image.{to_svg, wedge}

pub fn smain() {
  wedge(40, 270, fill.blue)
  |> to_svg
}
