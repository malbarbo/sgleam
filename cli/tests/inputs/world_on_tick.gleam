import gleam/int
import gleam/io
import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn draw(state: Int) -> image.Image {
  image.circle(10 + state, stroke.red)
}

pub fn tick(state: Int) -> Int {
  io.println("tick " <> int.to_string(state + 1))
  state + 1
}

pub fn main() {
  world.create(0, draw)
  |> world.tick_rate(100)
  |> world.on_tick(tick)
  |> world.stop_when(fn(state) { state == 3 })
  |> world.run()
}
