import sgleam/image

pub fn smain() {
  case image.bitmap("bitmap_not_image.gleam") {
    Ok(_) -> "Ok"
    Error(reason) -> reason
  }
}
