import gleam/io
import sgleam/image.{rectangle, to_svg, underlay_xy}
import sgleam/stroke

pub fn main() {
  rectangle(20, 20, stroke.black)
  |> underlay_xy(20, 0, rectangle(20, 20, stroke.black))
  |> to_svg
  |> io.println
}
