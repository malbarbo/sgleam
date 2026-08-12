import gleam/io
import sgleam/fill
import sgleam/image.{above, flip_vertical, scale_xyf, star, to_svg}

pub fn main() {
  star(40, fill.firebrick)
  |> above(star(40, fill.gray) |> flip_vertical |> scale_xyf(1.0, 0.5))
  |> to_svg
  |> io.println
}
