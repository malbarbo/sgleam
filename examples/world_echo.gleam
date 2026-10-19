import sgleam/fill
import sgleam/image
import sgleam/world

/// A world that grows a square at each tick and shows each tick and each key
/// with `echo`. In the terminal, the last lines show under the square.
pub fn smain() {
  world.create(0, fn(size) { image.square(20 + size, fill.red) })
  |> world.tick_rate(2)
  |> world.on_tick(fn(size) { echo size + 1 as "tick" })
  |> world.on_key_press(fn(size, key) {
    echo key as "key"
    size
  })
  |> world.stop_when(fn(size) { size == 20 })
}
