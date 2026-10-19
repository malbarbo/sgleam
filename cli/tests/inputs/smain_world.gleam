import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn draw(state: Int) -> image.Image {
  image.circle(10 + state, stroke.red)
}

pub fn tick(state: Int) -> Int {
  echo state + 1 as "tick"
}

pub fn smain() {
  world.create(0, draw)
  |> world.tick_rate(100)
  |> world.on_tick(tick)
  |> world.stop_when(fn(state) { state == 3 })
}
