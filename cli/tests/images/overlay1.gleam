import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, overlay, rectangle, to_svg}

pub fn main() {
  rectangle(30, 60, fill.orange)
  |> overlay(ellipse(60, 30, fill.purple))
  |> to_svg
  |> io.println
}
