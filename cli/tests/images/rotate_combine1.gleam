import gleam/io
import sgleam/fill
import sgleam/image.{beside, combine, rectangle, rotate, to_svg}

pub fn main() {
  combine([rectangle(40, 20, fill.red)], beside)
  |> rotate(90)
  |> to_svg
  |> io.println
}
