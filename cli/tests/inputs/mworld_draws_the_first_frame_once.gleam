import sgleam/image
import sgleam/mworld
import sgleam/stroke

pub fn draw(state: Int, player: mworld.Player) -> image.Image {
  echo player as "draw"
  image.circle(10 + state, stroke.red)
}

pub fn smain() {
  mworld.create(0, min: 1, max: 1, to_image: draw)
  |> mworld.stop_when(fn(_) { True })
}
