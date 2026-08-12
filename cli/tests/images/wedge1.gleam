import gleam/io
import sgleam/image.{to_svg, wedge}
import sgleam/fill

pub fn main() {
  wedge(40, 90, fill.red)
  |> to_svg
  |> io.println
}
