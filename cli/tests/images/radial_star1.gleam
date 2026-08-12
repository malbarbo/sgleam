import gleam/io
import sgleam/fill
import sgleam/image.{radial_star, to_svg}

pub fn main() {
  radial_star(8, 8, 64, fill.darkslategray)
  |> to_svg
  |> io.println
}
