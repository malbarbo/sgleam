import gleam/int
import gleam/list
import gleam/string
import sgleam/fill
import sgleam/image.{type Image}

fn show(img: Image) -> String {
  int.to_string(image.width(img)) <> "x" <> int.to_string(image.height(img))
}

pub fn smain() {
  let s = fill.black
  [
    image.regular_polygon(10, 2, s),
    image.regular_polygon(10, -3, s),
    image.regular_polygon(10, 3, s),
    image.star_polygon(10, 6, 2, s),
    image.star_polygon(10, 5, 0, s),
    image.star_polygon(10, 5, 5, s),
    image.star_polygon(10, 5, 2, s),
    image.radial_star(1, 5, 10, s),
    image.radial_star(2, 5, 10, s),
    image.pulled_regular_polygon(10, 2, 0.5, 30.0, s),
    image.pulled_regular_polygon(10, 3, 0.5, 30.0, s),
  ]
  |> list.map(show)
  |> string.join("\n")
}
