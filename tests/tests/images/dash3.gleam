import gleam/io
import sgleam/image.{line, to_svg}
import sgleam/stroke
import sgleam/style

pub fn main() {
  line(100, 0, [stroke.black, stroke.dash_dot, stroke.width(2)] |> style.join)
  |> to_svg
  |> io.println
}
