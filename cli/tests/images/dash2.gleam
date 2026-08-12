import gleam/io
import sgleam/image.{rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn main() {
  rectangle(80, 40, [stroke.blue, stroke.dotted] |> style.join)
  |> to_svg
  |> io.println
}
