import gleam/io
import sgleam/fill
import sgleam/image.{rectangle, to_svg, underlay_xy}

pub fn main() {
  rectangle(20, 20, fill.red)
  |> underlay_xy(-10, -10, rectangle(20, 20, fill.black))
  |> to_svg
  |> io.println
}
