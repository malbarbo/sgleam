import sgleam/fill
import sgleam/image.{to_svg, wedge}

pub fn smain() {
  wedge(40, 90, fill.red)
  |> to_svg
}
