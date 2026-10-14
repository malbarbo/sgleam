import gleam/int
import sgleam/fill
import sgleam/image
import sgleam/mworld

/// A shared counter with a view that identifies each player. Run it through a
/// sinteract room with two to four participants.
pub fn main() {
  mworld.create(0, min: 2, max: 4, to_image: draw)
  |> mworld.on_pad_down(fn(count, player, input) {
    echo #(mworld.player_index(player), input) as "input"
    count + 1
  })
  |> mworld.run()
}

fn draw(count: Int, player: mworld.Player) -> image.Image {
  let nickname = case mworld.player_nickname(player) {
    "" -> "Player " <> int.to_string(mworld.player_index(player))
    nickname -> nickname
  }
  image.text(nickname <> ": " <> int.to_string(count), 30, fill.black)
}
