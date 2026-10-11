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
    image.triangle_sss(1, 1, 10, s),
    image.triangle_sss(10, 1, 1, s),
    image.triangle_sss(1, 1, 2, s),
    image.triangle_aas(100, 100, 10, s),
    image.triangle_aas(90, 90, 10, s),
    image.triangle_aas(-30, 60, 10, s),
    image.triangle_aas(0, 60, 10, s),
    image.triangle_aas(60, 120, 10, s),
    image.triangle_aas(30, 60, 10, s),
    image.triangle_asa(120, 10, 120, s),
    image.triangle_asa(0, 10, 60, s),
    image.triangle_asa(-10, 10, 60, s),
    image.triangle_asa(60, 10, 60, s),
    image.triangle_saa(10, 90, 90, s),
    image.triangle_saa(10, 0, 60, s),
    image.triangle_saa(10, 0, 0, s),
    image.triangle_aas(0, 0, 10, s),
    image.triangle_asa(0, 10, 0, s),
    image.triangle_saa(10, 60, 60, s),
  ]
  |> list.map(show)
  |> string.join("\n")
}
