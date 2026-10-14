import sgleam/image.{type Image}
import sgleam/mworld

/// Runs an animation as a one-player multiplayer world.
pub fn animate(create_image: fn(Int) -> Image) -> Nil {
  create(0, create_image)
  |> tick_rate(28)
  |> on_tick(fn(frame) { frame + 1 })
  |> run
}

pub type KeyEvent {
  KeyEvent(
    event_type: KeyEventType,
    key: Key,
    alt: Bool,
    ctrl: Bool,
    shift: Bool,
    meta: Bool,
  )
}

pub type Key {
  ArrowLeft
  ArrowRight
  ArrowUp
  ArrowDown
  PageUp
  PageDown
  Home
  End
  Backspace
  Tab
  Enter
  Escape
  Delete
  Insert
  F1
  F2
  F3
  F4
  F5
  F6
  F7
  F8
  F9
  F10
  F11
  F12
  CapsLock
  NumLock
  ScrollLock
  PrintScreen
  Pause
  Shift
  Control
  Alt
  Meta
  Char(String)
}

pub type KeyEventType {
  KeyPress
  KeyDown
  KeyUp
}

/// A direction or action of the six-button teaching-game pad.
pub type PadInput {
  Up
  Down
  Left
  Right
  ActionA
  ActionB
}

pub const up = Up

pub const down = Down

pub const left = Left

pub const right = Right

pub const action_a = ActionA

pub const action_b = ActionB

pub type OnTick(a) =
  fn(a) -> a

pub type StopWhen(a) =
  fn(a) -> Bool

pub type OnKey(a) =
  fn(a, Key) -> a

pub type OnPad(a) =
  fn(a, PadInput) -> a

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
    mworld.on_key_press(inner, fn(state, _player, key) {
      handler(state, key_from_mworld(key))
    }),
  )
}

pub fn on_key_down(world: World(a), handler: OnKey(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_key_down(inner, fn(state, _player, key) {
      handler(state, key_from_mworld(key))
    }),
  )
}

pub fn on_key_up(world: World(a), handler: OnKey(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_key_up(inner, fn(state, _player, key) {
      handler(state, key_from_mworld(key))
    }),
  )
}

pub fn on_pad_down(world: World(a), handler: OnPad(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_pad_down(inner, fn(state, _player, input) {
      handler(state, pad_from_mworld(input))
    }),
  )
}

pub fn on_pad_up(world: World(a), handler: OnPad(a)) -> World(a) {
  let World(inner) = world
  World(
    mworld.on_pad_up(inner, fn(state, _player, input) {
      handler(state, pad_from_mworld(input))
    }),
  )
}

pub fn run(world: World(a)) {
  let World(inner) = world
  mworld.run(inner)
}

fn pad_from_mworld(input: mworld.PadInput) -> PadInput {
  case input {
    mworld.Up -> Up
    mworld.Down -> Down
    mworld.Left -> Left
    mworld.Right -> Right
    mworld.ActionA -> ActionA
    mworld.ActionB -> ActionB
  }
}

fn key_from_mworld(key: mworld.Key) -> Key {
  case key {
    mworld.ArrowLeft -> ArrowLeft
    mworld.ArrowRight -> ArrowRight
    mworld.ArrowUp -> ArrowUp
    mworld.ArrowDown -> ArrowDown
    mworld.PageUp -> PageUp
    mworld.PageDown -> PageDown
    mworld.Home -> Home
    mworld.End -> End
    mworld.Backspace -> Backspace
    mworld.Tab -> Tab
    mworld.Enter -> Enter
    mworld.Escape -> Escape
    mworld.Delete -> Delete
    mworld.Insert -> Insert
    mworld.F1 -> F1
    mworld.F2 -> F2
    mworld.F3 -> F3
    mworld.F4 -> F4
    mworld.F5 -> F5
    mworld.F6 -> F6
    mworld.F7 -> F7
    mworld.F8 -> F8
    mworld.F9 -> F9
    mworld.F10 -> F10
    mworld.F11 -> F11
    mworld.F12 -> F12
    mworld.CapsLock -> CapsLock
    mworld.NumLock -> NumLock
    mworld.ScrollLock -> ScrollLock
    mworld.PrintScreen -> PrintScreen
    mworld.Pause -> Pause
    mworld.Shift -> Shift
    mworld.Control -> Control
    mworld.Alt -> Alt
    mworld.Meta -> Meta
    mworld.Char(character) -> Char(character)
  }
}
