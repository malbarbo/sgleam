import gleam/io
import sgleam/fill
import sgleam/image.{to_svg, triangle_asa}

pub fn main() {
  triangle_asa(10, 200, 40, fill.seagreen)
  |> to_svg
  |> io.println
}
