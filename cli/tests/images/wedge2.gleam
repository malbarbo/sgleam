import gleam/io
import sgleam/image.{to_svg, wedge}
import sgleam/fill

pub fn main() {
  wedge(40, 270, fill.blue)
  |> to_svg
  |> io.println
}
