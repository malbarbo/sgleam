import gleam/io
import sgleam/fill
import sgleam/image.{to_svg, triangle_aas}

pub fn main() {
  triangle_aas(10, 40, 200, fill.seagreen)
  |> to_svg
  |> io.println
}
