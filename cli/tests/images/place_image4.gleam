import gleam/io
import sgleam/fill
import sgleam/image.{type Image, circle, empty_scene, place_image, to_svg}

pub fn main() {
  place(empty_scene(84, 12), 0)
  |> to_svg
  |> io.println
}

fn place(scene: Image, i: Int) -> Image {
  case i < 20 {
    True -> place(place_image(scene, 4 + i * 4, 6, circle(2, fill.red)), i + 1)
    False -> scene
  }
}
