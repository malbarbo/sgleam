import gleam/io
import sgleam/fill
import sgleam/image.{rhombus, to_svg}

pub fn main() {
  rhombus(40, 200, fill.mediumpurple)
  |> to_svg
  |> io.println
}
