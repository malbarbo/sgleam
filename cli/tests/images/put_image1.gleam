import gleam/io
import sgleam/fill
import sgleam/image.{ellipse, put_image, rectangle, to_svg}

pub fn main() {
  rectangle(50, 50, fill.lightgray)
  |> put_image(40, 15, ellipse(20, 30, fill.red))
  |> to_svg
  |> io.println
}
