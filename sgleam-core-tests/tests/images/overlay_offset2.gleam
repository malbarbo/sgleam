import gleam/io
import sgleam/fill
import sgleam/image.{circle, overlay_offset, rectangle, to_svg}

pub fn main() {
  rectangle(60, 20, fill.black)
  |> overlay_offset(-50, 0, circle(20, fill.darkorange))
  |> overlay_offset(70, 0, circle(20, fill.darkorange))
  |> to_svg
  |> io.println
}
