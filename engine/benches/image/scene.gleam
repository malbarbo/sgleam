import bench
import gleam/list
import gleam/string
import sgleam/fill
import sgleam/image

pub fn smain() {
  let sizes = [100, 200, 400]
  [
    bench.run("place_image scene", sizes, scene),
    bench.run_on("to_svg of the scene", sizes, scene, image.to_svg),
    bench.run("place_image text scene", sizes, text_scene),
    bench.run_on("to_svg of the text scene", sizes, text_scene, image.to_svg),
  ]
  |> string.join("\n")
}

fn scene(n: Int) -> image.Image {
  bench.range(n)
  |> list.fold(image.empty_scene(800, 600), fn(scene, i) {
    image.place_image(
      scene,
      i * 7 % 800,
      i * 13 % 600,
      image.square(8, fill.red),
    )
  })
}

fn text_scene(n: Int) -> image.Image {
  bench.range(n)
  |> list.fold(image.empty_scene(800, 600), fn(scene, i) {
    image.place_image(
      scene,
      i * 7 % 800,
      i * 13 % 600,
      image.text("score", 12, fill.black),
    )
  })
}
