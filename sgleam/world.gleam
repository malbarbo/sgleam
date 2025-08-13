import gleam/int
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

pub type OnTick(a) =
  fn(a) -> a

pub type StopWhen(a) =
  fn(a) -> Bool

pub type OnKey(a) =
  fn(a, Key) -> a

pub type ToImage(a) =
  fn(a) -> Image

const min_tick_rate = 1

const max_tick_rate = 1000

const default_tick_rate = 28

const key_event_pooling_delay = 10

pub opaque type World(a) {
  World(
    state: a,
    to_image: ToImage(a),
    rate: Int,
    on_tick: Option(OnTick(a)),
    stop_when: Option(StopWhen(a)),
    on_key_press: Option(OnKey(a)),
    on_key_down: Option(OnKey(a)),
    on_key_up: Option(OnKey(a)),
  )
}

pub fn create(state: a, to_image: ToImage(a)) -> World(a) {
  World(
    state:,
    to_image:,
    rate: default_tick_rate,
    on_tick: None,
    stop_when: None,
    on_key_press: None,
    on_key_down: None,
    on_key_up: None,
  )
}

pub fn tick_rate(world: World(a), rate: Int) -> World(a) {
  World(..world, rate: int.clamp(rate, min_tick_rate, max_tick_rate))
}

pub fn on_tick(world: World(a), handler: OnTick(a)) -> World(a) {
  World(..world, on_tick: Some(handler))
}

pub fn stop_when(world: World(a), handler: StopWhen(a)) -> World(a) {
  World(..world, stop_when: Some(handler))
}

pub fn on_key_press(world: World(a), handler: OnKey(a)) -> World(a) {
  World(..world, on_key_press: Some(handler))
}

pub fn on_key_down(world: World(a), handler: OnKey(a)) -> World(a) {
  World(..world, on_key_down: Some(handler))
}

pub fn on_key_up(world: World(a), handler: OnKey(a)) -> World(a) {
  World(..world, on_key_up: Some(handler))
}

pub fn run(world: World(a)) {
  world.state |> world.to_image |> image.to_svg |> system.show_svg
  run_loop(world, max_tick_rate / world.rate)
}

fn run_loop(world: World(a), time_out: Int) {
  let #(world, time_out) = case time_out <= 0 {
    True -> {
      let world = case world.on_tick {
        Some(on_tick) -> World(..world, state: on_tick(world.state))
        None -> world
      }
      show_svg(world)
      #(world, max_tick_rate / world.rate)
    }
    False -> #(world, time_out - key_event_pooling_delay)
  }
  let event = get_key_event()
  let world = case world.on_key_down, event {
    Some(on_key_down), Some(event) if event.event_type == KeyDown ->
      World(..world, state: on_key_down(world.state, event.key))
    _, _ -> world
  }
  let world = case world.on_key_press, event {
    Some(on_key_press), Some(event) if event.event_type == KeyPress ->
      World(..world, state: on_key_press(world.state, event.key))
    _, _ -> world
  }
  let world = case world.on_key_up, event {
    Some(on_key_up), Some(event) if event.event_type == KeyUp ->
      World(..world, state: on_key_up(world.state, event.key))
    _, _ -> world
  }
  case world.stop_when |> option.map(fn(f) { f(world.state) }) {
    Some(True) -> show_svg(world)
    _ -> {
      system.sleep(key_event_pooling_delay)
      run_loop(world, time_out)
    }
  }
}

fn show_svg(world: World(a)) {
  world.state |> world.to_image |> image.to_svg |> system.show_svg
}

fn get_key_event() -> Option(KeyEvent) {
  let event = system.get_key_event()
  case event {
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
    "ArrowLeft" -> ArrowLeft
    "ArrowRight" -> ArrowRight
    "ArrowUp" -> ArrowUp
    "ArrowDown" -> ArrowDown
    "PageUp" -> PageUp
    "PageDown" -> PageDown
    "Home" -> Home
    "End" -> End
    "Backspace" -> Backspace
    "Tab" -> Tab
    "Enter" -> Enter
    "Escape" -> Escape
    "Delete" -> Delete
    "Insert" -> Insert
    "F1" -> F1
    "F2" -> F2
    "F3" -> F3
    "F4" -> F4
    "F5" -> F5
    "F6" -> F6
    "F7" -> F7
    "F8" -> F8
    "F9" -> F9
    "F10" -> F10
    "F11" -> F11
    "F12" -> F12
    "CapsLock" -> CapsLock
    "NumLock" -> NumLock
    "ScrollLock" -> ScrollLock
    "PrintScreen" -> PrintScreen
    "Pause" -> Pause
    "Shift" -> Shift
    "Control" -> Control
    "Alt" -> Alt
    "Meta" -> Meta
    other -> Char(other)
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
