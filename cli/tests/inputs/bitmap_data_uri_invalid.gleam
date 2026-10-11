import gleam/int
import sgleam/image

pub fn smain() {
  let img = image.bitmap_data_uri("data:image/png;base64,AAAA")
  int.to_string(image.width(img)) <> "x" <> int.to_string(image.height(img))
}
