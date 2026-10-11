import sgleam/image.{to_svg, wedge}
import sgleam/fill

pub fn smain() {
  wedge(40, 90, fill.red)
  |> to_svg
}
