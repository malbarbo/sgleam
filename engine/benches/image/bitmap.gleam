import bench
import gleam/list
import sgleam/image

/// A 16 by 16 png.
const png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAABJ0lEQVR42hXMwYBFIQAAwIfwEUIIIYQQQgghhBBCCCGEEELIYHc6zHW+72t/PwKRRKZQaXQGk8XmcPm+n4BAJJEpVBqdwWSxOdzfC4KAQCSRKVQancFksTnc8IIoIBBJZAqVRmcwWWwON74gCQhEEplCpdEZTBabw00vyAICkUSmUGl0BpPF5nDzC4qAQCSRKVQancFksTnc8oIqIBBJZAqVRmcwWWwOt76gCQhEEplCpdEZTBabw20v6AICkUSmUGl0BpPF5nD7C4aAQCSRKVQancFksTnc8YIpIBBJZAqVRmcwWWwOd75gCQhEEplCpdEZTBabw10v2AICkUSmUGl0BpPF5nD3C46AQCSRKVQancFksTnc84IrIBBJZAqVRmcwWWwOl39OS28fFvYVWwAAAABJRU5ErkJggg=="

pub fn main() {
  let sizes = [25, 50, 100]
  bench.run("bitmap_data_uri", [1], fn(_) { image.bitmap_data_uri(png) })
  bench.run_on("to_svg of a bitmap scene", sizes, scene, image.to_svg)
}

fn scene(n: Int) -> image.Image {
  let bitmap = image.bitmap_data_uri(png)
  bench.range(n)
  |> list.fold(image.empty_scene(800, 600), fn(scene, i) {
    image.place_image(scene, i * 7 % 800, i * 13 % 600, bitmap)
  })
}
