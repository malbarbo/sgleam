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
  show(image.triangle_sss(1, 1, 10, s))
  show(image.triangle_sss(10, 1, 1, s))
  show(image.triangle_sss(1, 1, 2, s))
  show(image.triangle_aas(100, 100, 10, s))
  show(image.triangle_aas(90, 90, 10, s))
  show(image.triangle_aas(-30, 60, 10, s))
  show(image.triangle_aas(0, 60, 10, s))
  show(image.triangle_aas(60, 120, 10, s))
  show(image.triangle_aas(30, 60, 10, s))
  show(image.triangle_asa(120, 10, 120, s))
  show(image.triangle_asa(0, 10, 60, s))
  show(image.triangle_asa(-10, 10, 60, s))
  show(image.triangle_asa(60, 10, 60, s))
  show(image.triangle_saa(10, 90, 90, s))
  show(image.triangle_saa(10, 0, 60, s))
  show(image.triangle_saa(10, 60, 60, s))
}
