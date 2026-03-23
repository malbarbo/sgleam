import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn draw(_state: Int) -> image.Image {
  image.circle(30, stroke.red)
}

pub fn main() {
  world.create(0, draw)
  |> world.stop_when(fn(_) { True })
  |> world.run()
}
