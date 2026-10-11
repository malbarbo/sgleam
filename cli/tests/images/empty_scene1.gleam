import sgleam/image.{empty_scene, to_svg}

pub fn smain() {
  empty_scene(160, 90)
  |> to_svg
}
