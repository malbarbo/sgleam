import gleam/io
import sgleam/image

pub fn main() {
  case image.bitmap("bitmap_not_image.gleam") {
    Ok(_) -> io.println("Ok")
    Error(reason) -> io.println(reason)
  }
}
