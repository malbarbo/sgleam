import gleam/io
import sgleam/fill
import sgleam/image.{to_svg, triangle_ass}

pub fn main() {
  triangle_ass(10, 60, 100, fill.seagreen)
  |> to_svg
  |> io.println
}
