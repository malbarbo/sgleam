import sgleam/image.{to_svg, wedge}
import sgleam/fill

pub fn smain() {
  wedge(40, 360, fill.blue)
  |> to_svg
}
