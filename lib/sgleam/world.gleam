import gleam/int
import sgleam/image.{type Image}
import sgleam/key
import sgleam/mworld
import sgleam/pad
import sgleam/system

pub opaque type Animation {
  Animation(create_image: fn(Int) -> Image)
}

pub fn animation(create_image: fn(Int) -> Image) -> Animation {
  Animation(create_image)
}

/// An animation, so the FFI can take the class of an animation from it.
@internal
pub fn animation_variant() -> Animation {
  animation(fn(_) { image.empty })
}

/// Runs `animation`. The repl and `smain` call it for an animation they
/// return.
@internal
pub fn run_animation(animation: Animation) -> Nil {
  animate_loop(animation.create_image, 1000 / 28, 0)
  close_display()
}

fn animate_loop(create_image: fn(Int) -> Image, delay: Int, frame: Int) -> Nil {
  frame |> create_image |> show_image
  case closed_before(system.now_ms() + delay) {
    True -> Nil
    False -> animate_loop(create_image, delay, frame + 1)
  }
}

/// Returns `True` if the display closes before `deadline`, `False` otherwise.
/// The keys that arrive meanwhile go unused.
fn closed_before(deadline: Int) -> Bool {
  case system.wait_event(int.max(deadline - system.now_ms(), 0)) {
    ["close"] -> True
    [] -> False
    _ -> closed_before(deadline)
  }
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

/// A world, so the FFI can take the class of a world from it. The
/// constructor of an opaque type stays out of the exports of the module.
@internal
pub fn variant() -> World(Nil) {
  create(Nil, fn(_) { image.empty })
}

/// Runs `world`. The repl and `smain` call it for a world they return.
@internal
pub fn run(world: World(a)) {
  let World(inner) = world
  mworld.run(inner)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_image")
fn show_image(image: Image) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "close_display")
fn close_display() -> Nil
