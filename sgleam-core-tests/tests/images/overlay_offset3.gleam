import gleam/io
import sgleam/fill
import sgleam/image.{circle, overlay_offset, to_svg}

pub fn main() {
  circle(30, fill.rgba(0, 150, 0, 0.5))
  |> overlay_offset(26, 0, circle(30, fill.rgba(0, 0, 255, 0.5)))
  |> overlay_offset(0, 26, circle(30, fill.rgba(200, 0, 0, 0.5)))
  |> to_svg
  |> io.println
}
