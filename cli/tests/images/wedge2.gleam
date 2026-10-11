import sgleam/image.{to_svg, wedge}
import sgleam/fill

pub fn smain() {
  wedge(40, 270, fill.blue)
  |> to_svg
}
