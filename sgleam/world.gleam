import gleam/list
import gleam/option.{type Option, None, Some}
import sgleam/image.{type Image}
import sgleam/system

pub type KeyEvent {
  KeyEvent(
    event_type: KeyEventType,
    key: Key,
    alt: Bool,
    ctrl: Bool,
    shift: Bool,
    meta: Bool,
    repeat: Bool,
  )
}

pub type Key {
  Left
  Right
  Up
  Down
  Char(String)
}

pub type KeyEventType {
  KeyPress
  KeyDown
  KeyUp
}

pub type OnKey(a) =
  fn(a, Key) -> a

pub type ToImage(a) =
  fn(a) -> Image

pub opaque type World(a) {
  World(state: a, to_image: ToImage(a), on_key: Option(OnKey(a)))
}

pub fn create(state: a, to_image: ToImage(a)) -> World(a) {
  World(state:, to_image:, on_key: None)
}

pub fn run(world: World(a)) {
  world.state |> world.to_image |> image.to_svg |> system.show_svg
  let world = case world.on_key, get_key_event() {
    Some(on_key), Some(event) ->
      World(..world, state: on_key(world.state, event.key))
    _, _ -> world
  }
  system.sleep(1)
  run(world)
}

pub fn on_key(world: World(a), handler: OnKey(a)) -> World(a) {
  World(..world, on_key: Some(handler))
}

fn get_key_event() -> Option(KeyEvent) {
  case system.get_key_event() {
    ["keypress", key, ..modifiers] ->
      Some(new_key_event(KeyPress, key, modifiers))
    ["keydown", key, ..modifiers] ->
      Some(new_key_event(KeyDown, key, modifiers))
    ["keyup", key, ..modifiers] -> Some(new_key_event(KeyUp, key, modifiers))
    _ -> None
  }
}

fn new_key_event(
  event_type: KeyEventType,
  key: String,
  modifiers: List(String),
) -> KeyEvent {
  let key = case key {
    "Left" -> Left
    "Right" -> Right
    "Up" -> Up
    "Down" -> Down
    _ -> Char(key)
  }
  KeyEvent(
    event_type,
    key,
    alt: list.contains(modifiers, "alt"),
    ctrl: list.contains(modifiers, "ctrl"),
    shift: list.contains(modifiers, "shift"),
    meta: list.contains(modifiers, "meta"),
    repeat: list.contains(modifiers, "repeat"),
  )
}
