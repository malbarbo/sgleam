import gleam/io
import sgleam/fill
import sgleam/image

pub fn main() {
  image.rectangle(100, 200, fill.black)
  |> image.beside(image.rectangle(200, 100, fill.blue))
  |> image.scale(2)
  |> image.to_svg()
  |> io.println()
}
