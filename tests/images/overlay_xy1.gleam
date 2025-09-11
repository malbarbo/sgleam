import gleam/io
import sgleam/image.{overlay_xy, rectangle, to_svg}
import sgleam/stroke

pub fn main() {
  rectangle(20, 20, stroke.black)
  |> overlay_xy(20, 0, rectangle(20, 20, stroke.black))
  |> to_svg
  |> io.println
}
