import gleam/io
import sgleam/fill
import sgleam/image.{square, to_svg}

pub fn main() {
  square(40, fill.slateblue)
  |> to_svg
  |> io.println
}
