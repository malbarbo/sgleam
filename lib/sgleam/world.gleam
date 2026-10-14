import sgleam/image.{type Image}
import sgleam/key
import sgleam/mworld
import sgleam/pad

/// Runs an animation as a one-player multiplayer world.
pub fn animate(create_image: fn(Int) -> Image) -> Nil {
  create(0, create_image)
  |> tick_rate(28)
  |> on_tick(fn(frame) { frame + 1 })
  |> run
}

pub type OnTick(a) =
  fn(a) -> a

pub type StopWhen(a) =
  fn(a) -> Bool

pub type OnKey(a) =
  fn(a, key.Event) -> a

pub type OnPad(a) =
  fn(a, pad.Event) -> a

pub type ToImage(a) =
  fn(a) -> Image

/// A `MultiWorld` specialized to exactly one player. Its handlers hide the
/// player value, preserving the original single-player API.
pub opaque type World(a) {
  World(mworld.MultiWorld(a))
}

pub fn create(state: a, to_image: ToImage(a)) -> World(a) {
  World(
    mworld.create(state, min: 1, max: 1, to_image: fn(state, _player) {
      to_image(state)
    })
    |> mworld.reuse_preview,
  )
}

pub fn tick_rate(world: World(a), rate: Int) -> World(a) {
  let World(inner) = world
  World(mworld.tick_rate(inner, rate))
}

pub fn on_tick(world: World(a), handler: OnTick(a)) -> World(a) {
  let World(inner) = world
  World(mworld.on_tick(inner, handler))
}

pub fn stop_when(world: World(a), handler: StopWhen(a)) -> World(a) {
  let World(inner) = world
  World(mworld.stop_when(inner, handler))
}

pub fn on_key_press(world: World(a), handler: OnKey(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_key_press(inner, fn(state, _player, event) {
      handler(state, event)
    }),
  )
}

pub fn on_key_down(world: World(a), handler: OnKey(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_key_down(inner, fn(state, _player, event) {
      handler(state, event)
    }),
  )
}

pub fn on_key_up(world: World(a), handler: OnKey(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_key_up(inner, fn(state, _player, event) { handler(state, event) }),
  )
}

pub fn on_pad_down(world: World(a), handler: OnPad(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_pad_down(inner, fn(state, _player, event) {
      handler(state, event)
    }),
  )
}

pub fn on_pad_up(world: World(a), handler: OnPad(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_pad_up(inner, fn(state, _player, event) { handler(state, event) }),
  )
}

pub fn run(world: World(a)) {
  let World(inner) = world
  mworld.run(inner)
}
