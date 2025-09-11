import gleam/io
import sgleam/fill
import sgleam/image.{radial_start, to_svg}

pub fn main() {
  radial_start(8, 8, 64, fill.darkslategray)
  |> to_svg
  |> io.println
}
