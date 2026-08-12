import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, to_svg, underlay_xy}

pub fn main() {
  ellipse(40, 40, fill.lightgray)
  |> underlay_xy(10, 15, ellipse(10, 10, fill.forestgreen))
  |> underlay_xy(20, 15, ellipse(10, 10, fill.forestgreen))
  |> to_svg
  |> io.println
}
