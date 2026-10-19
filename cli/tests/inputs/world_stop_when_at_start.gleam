import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn draw(_state: Int) -> image.Image {
  image.circle(20, stroke.red)
}

pub fn tick(state: Int) -> Int {
  echo state + 1 as "tick"
}

// A tick a second, so the stop is what the first pass of the loop finds.
pub fn smain() {
  world.create(0, draw)
  |> world.tick_rate(1)
  |> world.on_tick(tick)
  |> world.stop_when(fn(state) { state == 0 })
}
