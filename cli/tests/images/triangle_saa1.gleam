import gleam/io
import sgleam/fill
import sgleam/image.{to_svg, triangle_saa}

pub fn main() {
  triangle_saa(100, 10, 40, fill.seagreen)
  |> to_svg
  |> io.println
}
