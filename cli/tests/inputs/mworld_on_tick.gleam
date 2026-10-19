import sgleam/image
import sgleam/mworld
import sgleam/stroke

pub fn draw(state: Int, _player: mworld.Player) -> image.Image {
  image.circle(10 + state, stroke.red)
}

pub fn tick(state: Int) -> Int {
  echo state + 1 as "tick"
}

pub fn smain() {
  mworld.create(0, min: 1, max: 1, to_image: draw)
  |> mworld.tick_rate(100)
  |> mworld.on_tick(tick)
  |> mworld.stop_when(fn(state) { state == 3 })
}
