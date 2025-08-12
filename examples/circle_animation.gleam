import sgleam/create
import sgleam/fill
import sgleam/image

pub fn main() {
  create.animation(circle, 10)
}

const max = 200

fn circle(n: Int) {
  let n = n % max
  let n = case n < max / 2 {
    True -> n
    False -> max - n
  }
  image.empty_scene(max, max)
  |> image.overlay(image.circle(n, fill.red))
}
