import gleam/io
import sgleam/fill
import sgleam/image.{overlay, rectangle, text, to_svg}

pub fn main() {
  overlay(text("Hello", 20, fill.white), rectangle(80, 30, fill.blue))
  |> to_svg
  |> io.println
}
