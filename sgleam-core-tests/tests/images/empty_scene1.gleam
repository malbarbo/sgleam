import gleam/io
import sgleam/image.{empty_scene, to_svg}

pub fn main() {
  empty_scene(160, 90)
  |> to_svg
  |> io.println
}
