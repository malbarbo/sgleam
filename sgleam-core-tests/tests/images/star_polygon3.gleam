import gleam/io
import sgleam/fill
import sgleam/image.{star_polygon, to_svg}

pub fn main() {
  star_polygon(20, 10, 3, fill.cornflowerblue)
  |> to_svg
  |> io.println
}
