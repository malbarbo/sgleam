import gleam/result
import sgleam/mworld

pub fn smain() {
  let player =
    mworld.fake_player(2, "ana")
    |> result.map(fn(player) {
      #(mworld.player_index(player), mworld.player_nickname(player))
    })
  #(
    player,
    mworld.fake_player(0, "bia") |> result.is_error,
    mworld.fake_player(1025, "caio") |> result.is_error,
  )
}
