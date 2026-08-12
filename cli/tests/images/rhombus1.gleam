import gleam/io
import sgleam/fill
import sgleam/image.{rhombus, to_svg}

pub fn main() {
  rhombus(40, 45, fill.magenta)
  |> to_svg
  |> io.println
}
