import gleam/int
import gleam/io
import sgleam/image

pub fn main() {
  let img = image.bitmap_data_uri("data:image/png;base64,AAAA")
  io.println(
    int.to_string(image.width(img)) <> "x" <> int.to_string(image.height(img)),
  )
}
