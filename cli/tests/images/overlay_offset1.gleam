import gleam/io
import sgleam/fill
import sgleam/image.{circle, overlay_offset, to_svg}

pub fn main() {
  circle(40, fill.red)
  |> overlay_offset(10, 10, circle(40, fill.blue))
  |> to_svg
  |> io.println
}
