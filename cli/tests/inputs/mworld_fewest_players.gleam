import sgleam/image
import sgleam/mworld
import sgleam/stroke

pub fn draw(state: Int, player: mworld.Player) -> image.Image {
  echo mworld.player_index(player) as "draw"
  image.circle(10 + state, stroke.red)
}

pub fn tick(state: Int) -> Int {
  state + 1
}

pub fn smain() {
  mworld.create(0, min: 2, max: 3, to_image: draw)
  |> mworld.tick_rate(100)
  |> mworld.on_tick(tick)
  |> mworld.stop_when(fn(state) { state == 2 })
}
