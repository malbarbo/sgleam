import gleam/bool
import gleam/io
import sgleam/image
import sgleam/stroke
import sgleam/system
import sgleam/world

pub fn draw(_state: Int) -> image.Image {
  image.circle(20, stroke.red)
}

/// The rate is clamped to [1, 100], so the ticks of a world take time a rate
/// taken as given would not: unclamped, both of these run in no time at all.
pub fn main() {
  // 1000 is clamped to 100: 5 ticks of 10ms.
  io.println("max: " <> bool.to_string(elapsed(1000, 5) >= 40))
  // 0 is clamped to 1: a tick of a second, and not a division by zero.
  io.println("min: " <> bool.to_string(elapsed(0, 1) >= 900))
}

fn elapsed(rate: Int, ticks: Int) -> Int {
  let start = system.now_ms()
  world.create(0, draw)
  |> world.tick_rate(rate)
  |> world.on_tick(fn(state) { state + 1 })
  |> world.stop_when(fn(state) { state == ticks })
  |> world.run()
  system.now_ms() - start
}
