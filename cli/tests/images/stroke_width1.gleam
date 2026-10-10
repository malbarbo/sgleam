import gleam/io
import sgleam/image.{rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn main() {
  rectangle(6, 4, style.join([stroke.black, stroke.width(1)]))
  |> to_svg
  |> io.println
}
