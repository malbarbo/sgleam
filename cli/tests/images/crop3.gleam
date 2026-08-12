import gleam/io
import sgleam/fill
import sgleam/image.{above, beside, circle, crop, rotate, to_svg}

pub fn main() {
  above(
    beside(
      circle(40, fill.palevioletred) |> crop(40, 40, 40, 40),
      circle(40, fill.lightcoral) |> crop(0, 40, 40, 40),
    ),
    beside(
      circle(40, fill.lightcoral) |> crop(40, 0, 40, 40),
      circle(40, fill.palevioletred) |> crop(0, 0, 40, 40),
    ),
  )
  |> rotate(30)
  |> to_svg
  |> io.println
}
