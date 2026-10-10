import gleam/int
import gleam/io
import sgleam/fill
import sgleam/image.{type Image}

fn show(img: Image) {
  io.println(
    int.to_string(image.width(img)) <> "x" <> int.to_string(image.height(img)),
  )
}

pub fn main() {
  let s = fill.black
  show(image.regular_polygon(10, 2, s))
  show(image.regular_polygon(10, -3, s))
  show(image.regular_polygon(10, 3, s))
  show(image.star_polygon(10, 6, 2, s))
  show(image.star_polygon(10, 5, 0, s))
  show(image.star_polygon(10, 5, 5, s))
  show(image.star_polygon(10, 5, 2, s))
  show(image.radial_star(1, 5, 10, s))
  show(image.radial_star(2, 5, 10, s))
  show(image.pulled_regular_polygon(10, 2, 0.5, 30.0, s))
  show(image.pulled_regular_polygon(10, 3, 0.5, 30.0, s))
}
