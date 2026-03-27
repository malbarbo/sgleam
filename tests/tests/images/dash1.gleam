import gleam/io
import sgleam/image.{circle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn main() {
  circle(30, [stroke.red, stroke.dashed] |> style.join)
  |> to_svg
  |> io.println
}
