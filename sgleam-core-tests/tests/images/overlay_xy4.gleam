import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, overlay_xy, to_svg}
import sgleam/stroke

pub fn main() {
  ellipse(40, 40, stroke.black)
  |> overlay_xy(10, 15, ellipse(10, 10, fill.forestgreen))
  |> overlay_xy(20, 15, ellipse(10, 10, fill.forestgreen))
  |> to_svg
  |> io.println
}
