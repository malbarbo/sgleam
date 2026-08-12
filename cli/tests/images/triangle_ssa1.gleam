import gleam/io
import sgleam/fill
import sgleam/image.{to_svg, triangle_ssa}

pub fn main() {
  triangle_ssa(60, 100, 10, fill.seagreen)
  |> to_svg
  |> io.println
}
