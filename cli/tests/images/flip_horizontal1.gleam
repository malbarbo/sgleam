import gleam/io
import sgleam/fill
import sgleam/image.{beside, flip_horizontal, rotate, square, to_svg}

pub fn main() {
  square(50, fill.red)
  |> rotate(30)
  |> beside(square(50, fill.blue) |> rotate(30) |> flip_horizontal)
  |> to_svg
  |> io.println
}
