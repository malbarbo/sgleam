import gleam/io
import sgleam/fill
import sgleam/image.{above, beside, circle, crop_align, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  above(
    beside(
      circle(40, fill.palevioletred)
        |> crop_align(xplace.Right, yplace.Bottom, 40, 40),
      circle(40, fill.lightcoral)
        |> crop_align(xplace.Left, yplace.Bottom, 40, 40),
    ),
    beside(
      circle(40, fill.lightcoral)
        |> crop_align(xplace.Right, yplace.Top, 40, 40),
      circle(40, fill.palevioletred)
        |> crop_align(xplace.Left, yplace.Top, 40, 40),
    ),
  )
  |> to_svg
  |> io.println
}
