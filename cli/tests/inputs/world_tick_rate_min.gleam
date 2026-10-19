import sgleam/image
import sgleam/stroke
import sgleam/system
import sgleam/world

pub fn draw(_state: #(Int, Int)) -> image.Image {
  image.circle(20, stroke.red)
}

/// The rate is clamped to [1, 100], so the ticks of a world take time a rate
/// taken as given would not. 0 is clamped to 1, so the tick takes a
/// second, and the rate is not a division by zero.
pub fn smain() {
  world.create(#(0, system.now_ms()), draw)
  |> world.tick_rate(0)
  |> world.on_tick(tick)
  |> world.stop_when(fn(state) { state.0 == 1 })
}

fn tick(state: #(Int, Int)) -> #(Int, Int) {
  let #(ticks, start) = state
  case ticks + 1 == 1 {
    True -> {
      echo system.now_ms() - start >= 900 as "min"
      Nil
    }
    False -> Nil
  }
  #(ticks + 1, start)
}
