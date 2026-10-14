import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import sgleam/image.{type Image}
import sgleam/system

/// A player fixed when the multiplayer world starts.
pub opaque type Player {
  Player(index: Int, nickname: String)
}

/// The one-based position of `player` in the match.
pub fn player_index(player: Player) -> Int {
  player.index
}

/// The nickname given for `player` when the match starts.
pub fn player_nickname(player: Player) -> String {
  player.nickname
}

/// Whether two player values identify the same participant.
pub fn same_player(first: Player, second: Player) -> Bool {
  first.index == second.index
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

pub type ToImage(a) =
  fn(a, Player) -> Image

pub type OnTick(a) =
  fn(a) -> a

pub type StopWhen(a) =
  fn(a) -> Bool

pub type OnKey(a) =
  fn(a, Player, Key) -> a

pub type OnPad(a) =
  fn(a, Player, PadInput) -> a

const min_tick_rate = 1

const max_tick_rate = 100

const default_tick_rate = 28

const ms_per_second = 1000

const max_player_count = 1024

pub opaque type MultiWorld(a) {
  MultiWorld(
    state: a,
    to_image: ToImage(a),
    rate: Int,
    min_players: Int,
    max_players: Int,
    players: List(Player),
    reuse_preview: Bool,
    on_tick: Option(OnTick(a)),
    stop_when: Option(StopWhen(a)),
    on_key_press: Option(OnKey(a)),
    on_key_down: Option(OnKey(a)),
    on_key_up: Option(OnKey(a)),
    on_pad_down: Option(OnPad(a)),
    on_pad_up: Option(OnPad(a)),
  )
}

/// Creates a world whose fixed player list is chosen when the match starts.
pub fn create(
  state: a,
  min min_players: Int,
  max max_players: Int,
  to_image to_image: ToImage(a),
) -> MultiWorld(a) {
  let min_players = int.clamp(min_players, 1, max_player_count)
  let max_players = int.clamp(max_players, min_players, max_player_count)
  MultiWorld(
    state:,
    to_image:,
    rate: default_tick_rate,
    min_players:,
    max_players:,
    players: [],
    reuse_preview: False,
    on_tick: None,
    stop_when: None,
    on_key_press: None,
    on_key_down: None,
    on_key_up: None,
    on_pad_down: None,
    on_pad_up: None,
  )
}

/// Marks the preview as the first view. `world` uses this because its drawing
/// function cannot observe the provisional player used to size the stage.
@internal
pub fn reuse_preview(world: MultiWorld(a)) -> MultiWorld(a) {
  MultiWorld(..world, reuse_preview: True)
}

pub fn tick_rate(world: MultiWorld(a), rate: Int) -> MultiWorld(a) {
  MultiWorld(..world, rate: int.clamp(rate, min_tick_rate, max_tick_rate))
}

pub fn on_tick(world: MultiWorld(a), handler: OnTick(a)) -> MultiWorld(a) {
  MultiWorld(..world, on_tick: Some(handler))
}

pub fn stop_when(world: MultiWorld(a), handler: StopWhen(a)) -> MultiWorld(a) {
  MultiWorld(..world, stop_when: Some(handler))
}

pub fn on_key_press(world: MultiWorld(a), handler: OnKey(a)) -> MultiWorld(a) {
  MultiWorld(..world, on_key_press: Some(handler))
}

pub fn on_key_down(world: MultiWorld(a), handler: OnKey(a)) -> MultiWorld(a) {
  MultiWorld(..world, on_key_down: Some(handler))
}

pub fn on_key_up(world: MultiWorld(a), handler: OnKey(a)) -> MultiWorld(a) {
  MultiWorld(..world, on_key_up: Some(handler))
}

pub fn on_pad_down(world: MultiWorld(a), handler: OnPad(a)) -> MultiWorld(a) {
  MultiWorld(..world, on_pad_down: Some(handler))
}

pub fn on_pad_up(world: MultiWorld(a), handler: OnPad(a)) -> MultiWorld(a) {
  MultiWorld(..world, on_pad_up: Some(handler))
}

/// Runs one shared state and draws its current view for every fixed player.
pub fn run(world: MultiWorld(a)) {
  let preview_player = Player(1, "")
  let preview = world.to_image(world.state, preview_player)
  let players =
    open_mworld(preview, world.min_players, world.max_players)
    |> players_from_start
  let world = MultiWorld(..world, players:)
  case world.reuse_preview, players {
    True, [player] -> show_player_image(player.index, preview)
    _, _ -> show_players(world)
  }
  let period = ms_per_second / world.rate
  run_loop(world, system.now_ms() + period)
  close_display()
}

type RuntimeEvent {
  KeyPress(Player, Key)
  KeyDown(Player, Key)
  KeyUp(Player, Key)
  PadDown(Player, PadInput)
  PadUp(Player, PadInput)
  NoEvent
}

fn run_loop(world: MultiWorld(a), next_tick_at: Int) -> Nil {
  case world.stop_when |> option.map(fn(stop) { stop(world.state) }) {
    Some(True) -> Nil
    _ ->
      case wait_mworld_event(int.max(next_tick_at - system.now_ms(), 0)) {
        ["close"] -> Nil
        event -> {
          let #(world, next_tick_at) =
            turn(world, runtime_event(world.players, event), next_tick_at)
          run_loop(world, next_tick_at)
        }
      }
  }
}

/// Handles a logical tick when one is due and one input event. A changed
/// shared state produces one view for each player, but only once per turn.
fn turn(
  world: MultiWorld(a),
  event: RuntimeEvent,
  next_tick_at: Int,
) -> #(MultiWorld(a), Int) {
  let now = system.now_ms()
  let #(world, ticked, next_tick_at) = case now >= next_tick_at {
    True -> {
      let #(world, ticked) = case world.on_tick {
        Some(on_tick) -> #(
          MultiWorld(..world, state: on_tick(world.state)),
          True,
        )
        None -> #(world, False)
      }
      // Schedule from the deadline to absorb minor overruns without drift.
      // A large overrun snaps ahead instead of spending turns catching up.
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
  let #(world, handled) = apply_event(world, event)
  case ticked || handled {
    True -> show_players(world)
    False -> Nil
  }
  #(world, next_tick_at)
}

fn apply_event(
  world: MultiWorld(a),
  event: RuntimeEvent,
) -> #(MultiWorld(a), Bool) {
  case event {
    KeyPress(player, key) ->
      apply_player_handler(world, player, key, world.on_key_press)
    KeyDown(player, key) ->
      apply_key_or_pad_handler(
        world,
        player,
        key,
        world.on_key_down,
        world.on_pad_down,
      )
    KeyUp(player, key) ->
      apply_key_or_pad_handler(
        world,
        player,
        key,
        world.on_key_up,
        world.on_pad_up,
      )
    PadDown(player, input) ->
      apply_player_handler(world, player, input, world.on_pad_down)
    PadUp(player, input) ->
      apply_player_handler(world, player, input, world.on_pad_up)
    NoEvent -> #(world, False)
  }
}

fn apply_key_or_pad_handler(
  world: MultiWorld(a),
  player: Player,
  key: Key,
  key_handler: Option(OnKey(a)),
  pad_handler: Option(OnPad(a)),
) -> #(MultiWorld(a), Bool) {
  case pad_input_from_key(key), pad_handler {
    Some(input), Some(handler) ->
      apply_player_handler(world, player, input, Some(handler))
    _, _ -> apply_player_handler(world, player, key, key_handler)
  }
}

fn apply_player_handler(
  world: MultiWorld(a),
  player: Player,
  input: input,
  handler: Option(fn(a, Player, input) -> a),
) -> #(MultiWorld(a), Bool) {
  case handler {
    Some(handler) -> #(
      MultiWorld(..world, state: handler(world.state, player, input)),
      True,
    )
    None -> #(world, False)
  }
}

fn show_players(world: MultiWorld(a)) {
  list.each(world.players, fn(player) {
    world.to_image(world.state, player)
    |> show_player_image(player.index, _)
  })
}

fn players_from_start(values: List(String)) -> List(Player) {
  case values {
    [index, nickname, ..rest] ->
      case int.parse(index) {
        Ok(index) if index > 0 -> [
          Player(index, nickname),
          ..players_from_start(rest)
        ]
        _ -> players_from_start(rest)
      }
    _ -> []
  }
}

fn runtime_event(players: List(Player), event: List(String)) -> RuntimeEvent {
  case event {
    ["keypress", player, key, ..] ->
      player_input(players, player, key_from_string(key), KeyPress)
    ["keydown", player, key, ..] ->
      player_input(players, player, key_from_string(key), KeyDown)
    ["keyup", player, key, ..] ->
      player_input(players, player, key_from_string(key), KeyUp)
    ["paddown", player, input] -> pad_event(players, player, input, PadDown)
    ["padup", player, input] -> pad_event(players, player, input, PadUp)
    _ -> NoEvent
  }
}

fn player_input(
  players: List(Player),
  index: String,
  input: input,
  event: fn(Player, input) -> RuntimeEvent,
) -> RuntimeEvent {
  case int.parse(index) {
    Ok(index) ->
      case find_player(players, index) {
        Some(player) -> event(player, input)
        None -> NoEvent
      }
    Error(_) -> NoEvent
  }
}

fn pad_event(
  players: List(Player),
  player: String,
  input: String,
  event: fn(Player, PadInput) -> RuntimeEvent,
) -> RuntimeEvent {
  case pad_input_from_string(input) {
    Some(input) -> player_input(players, player, input, event)
    None -> NoEvent
  }
}

fn find_player(players: List(Player), index: Int) -> Option(Player) {
  case list.find(players, fn(player) { player.index == index }) {
    Ok(player) -> Some(player)
    Error(_) -> None
  }
}

@internal
pub fn pad_input_from_string(input: String) -> Option(PadInput) {
  case input {
    "Up" -> Some(Up)
    "Down" -> Some(Down)
    "Left" -> Some(Left)
    "Right" -> Some(Right)
    "ActionA" -> Some(ActionA)
    "ActionB" -> Some(ActionB)
    _ -> None
  }
}

@internal
pub fn pad_input_from_key(key: Key) -> Option(PadInput) {
  case key {
    ArrowUp | Char("w") | Char("W") -> Some(Up)
    ArrowDown | Char("s") | Char("S") -> Some(Down)
    ArrowLeft | Char("a") | Char("A") -> Some(Left)
    ArrowRight | Char("d") | Char("D") -> Some(Right)
    Char("z") | Char("Z") | Char("k") | Char("K") -> Some(ActionA)
    Char("x") | Char("X") | Char("l") | Char("L") -> Some(ActionB)
    _ -> None
  }
}

@internal
pub fn key_from_string(key: String) -> Key {
  case key {
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
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "open_mworld")
fn open_mworld(
  preview: Image,
  min_players: Int,
  max_players: Int,
) -> List(String)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_player_image")
fn show_player_image(player: Int, image: Image) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "wait_mworld_event")
fn wait_mworld_event(ms: Int) -> List(String)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "close_display")
fn close_display() -> Nil
