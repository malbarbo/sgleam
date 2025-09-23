import sgleam/fill
import sgleam/image
import sgleam/world

pub fn main() {
  world.animate(circle)
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
