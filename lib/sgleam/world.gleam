import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import sgleam/image.{type Image}
import sgleam/system

pub fn animate(create_image: fn(Int) -> Image) -> Nil {
  animate_loop(create_image, 1000 / 28, 0)
}

fn animate_loop(create_image: fn(Int) -> Image, delay: Int, frame: Int) {
  frame |> create_image |> show_image
  system.sleep(delay)
  animate_loop(create_image, delay, frame + 1)
}

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

const max_tick_rate = 100

const default_tick_rate = 28

const ms_per_second = 1000

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
  show(world)
  let period = ms_per_second / world.rate
  run_loop(world, system.now_ms() + period)
  close_display()
}

fn run_loop(world: World(a), next_tick_at: Int) -> Nil {
  case world.stop_when |> option.map(fn(f) { f(world.state) }) {
    Some(True) -> Nil
    _ ->
      case system.wait_event(int.max(next_tick_at - system.now_ms(), 0)) {
        ["close"] -> Nil
        event -> {
          let #(world, next_tick_at) =
            turn(world, key_event(event), next_tick_at)
          run_loop(world, next_tick_at)
        }
      }
  }
}

/// Handles a tick when one is due and `event`, and draws once if any handler
/// ran. Returns the world and the time of the next tick.
fn turn(
  world: World(a),
  event: Option(KeyEvent),
  next_tick_at: Int,
) -> #(World(a), Int) {
  let now = system.now_ms()
  let #(world, ticked, next_tick_at) = case now >= next_tick_at {
    True -> {
      let #(world, ticked) = case world.on_tick {
        Some(on_tick) -> #(World(..world, state: on_tick(world.state)), True)
        None -> #(world, False)
      }
      // Schedule from the deadline (not from now) to absorb minor
      // overruns without drift. If we overran by more than a full
      // period, snap forward so we don't burn ticks catching up.
      let period = ms_per_second / world.rate
      let bumped = next_tick_at + period
      let next = case bumped <= now {
        True -> now + period
        False -> bumped
      }
      #(world, ticked, next)
    }
    False -> #(world, False, next_tick_at)
  }
  let #(world, h1) = case world.on_key_down, event {
    Some(handler), Some(event) if event.event_type == KeyDown -> #(
      World(..world, state: handler(world.state, event.key)),
      True,
    )
    _, _ -> #(world, False)
  }
  let #(world, h2) = case world.on_key_press, event {
    Some(handler), Some(event) if event.event_type == KeyPress -> #(
      World(..world, state: handler(world.state, event.key)),
      True,
    )
    _, _ -> #(world, False)
  }
  let #(world, h3) = case world.on_key_up, event {
    Some(handler), Some(event) if event.event_type == KeyUp -> #(
      World(..world, state: handler(world.state, event.key)),
      True,
    )
    _, _ -> #(world, False)
  }
  case ticked || h1 || h2 || h3 {
    True -> show(world)
    False -> Nil
  }
  #(world, next_tick_at)
}

fn show(world: World(a)) {
  world.state |> world.to_image |> show_image
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_image")
fn show_image(image: Image) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "close_display")
fn close_display() -> Nil

fn key_event(event: List(String)) -> Option(KeyEvent) {
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
