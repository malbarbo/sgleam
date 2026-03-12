import gleam/io
import sgleam/fill
import sgleam/image.{text, to_svg}

pub fn main() {
  text("Testing text", 16, fill.black)
  |> to_svg
  |> io.println
}
