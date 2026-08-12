import gleam/io
import sgleam/fill
import sgleam/image.{overlay_xy, rectangle, to_svg}

pub fn main() {
  rectangle(20, 20, fill.red)
  |> overlay_xy(-10, -10, rectangle(20, 20, fill.black))
  |> to_svg
  |> io.println
}
