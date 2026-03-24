import gleam/io
import sgleam/fill
import sgleam/image.{rectangle, to_svg}

pub fn main() {
  rectangle(20, 40, fill.black)
  |> to_svg
  |> io.println
}
