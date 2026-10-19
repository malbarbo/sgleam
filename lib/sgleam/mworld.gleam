import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import sgleam/image.{type Image}
import sgleam/key
import sgleam/pad
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

pub type ToImage(a) =
  fn(a, Player) -> Image

pub type OnTick(a) =
  fn(a) -> a

pub type StopWhen(a) =
  fn(a) -> Bool

pub type OnKey(a) =
  fn(a, Player, key.Event) -> a

pub type OnPad(a) =
  fn(a, Player, pad.Event) -> a

const min_tick_rate = 1

const max_tick_rate = 100

const default_tick_rate = 28

const ms_per_second = 1000

const max_player_count = 1024

type TickClock {
  TickClock(deadline: Int, remainder: Int)
}

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

/// A world, so the FFI can take the class of a world from it. The
/// constructor of an opaque type stays out of the exports of the module.
@internal
pub fn variant() -> MultiWorld(Nil) {
  create(Nil, min: 1, max: 1, to_image: fn(_, _) { image.empty })
}

/// Runs one shared state and draws its current view for every fixed player.
/// The repl and `smain` call it for a world they return.
@internal
pub fn run(world: MultiWorld(a)) {
  let preview = case mworld_uses_session(), world.min_players {
    False, 1 -> Some(world.to_image(world.state, Player(1, "")))
    _, _ -> None
  }
  let #(width, height) = case preview {
    Some(preview) -> #(image.width(preview), image.height(preview))
    None -> #(0, 0)
  }
  let players =
    open_mworld(width, height, world.min_players, world.max_players)
    |> players_from_start
  let world = MultiWorld(..world, players:)
  case world.reuse_preview, players, preview {
    True, [player], Some(preview) -> show_player_image(player.index, preview)
    _, _, _ -> show_players(world)
  }
  run_loop(world, start_tick_clock(system.now_ms(), world.rate))
  close_display()
}

type RuntimeEvent {
  KeyPress(Player, key.Event)
  KeyDown(Player, key.Event)
  KeyUp(Player, key.Event)
  PadDown(Player, pad.Event)
  PadUp(Player, pad.Event)
  NoEvent
}

fn run_loop(world: MultiWorld(a), clock: TickClock) -> Nil {
  case world.stop_when |> option.map(fn(stop) { stop(world.state) }) {
    Some(True) -> Nil
    _ ->
      case wait_mworld_event(int.max(clock.deadline - system.now_ms(), 0)) {
        ["close"] -> Nil
        event -> {
          let #(world, clock) =
            turn(world, runtime_event(world.players, event), clock)
          run_loop(world, clock)
        }
      }
  }
}

/// Handles a logical tick when one is due and one input event. A changed
/// shared state produces one view for each player, but only once per turn.
fn turn(
  world: MultiWorld(a),
  event: RuntimeEvent,
  clock: TickClock,
) -> #(MultiWorld(a), TickClock) {
  let now = system.now_ms()
  let #(world, ticked, clock) = case now >= clock.deadline {
    True -> {
      let #(world, ticked) = case world.on_tick {
        Some(on_tick) -> #(
          MultiWorld(..world, state: on_tick(world.state)),
          True,
        )
        None -> #(world, False)
      }
      let bumped = advance_tick_clock(clock, world.rate)
      let clock = case bumped.deadline <= now {
        True -> start_tick_clock(now, world.rate)
        False -> bumped
      }
      #(world, ticked, clock)
    }
    False -> #(world, False, clock)
  }
  let #(world, handled) = apply_event(world, event)
  case ticked || handled {
    True -> show_players(world)
    False -> Nil
  }
  #(world, clock)
}

fn start_tick_clock(now: Int, rate: Int) -> TickClock {
  advance_tick_clock(TickClock(deadline: now, remainder: 0), rate)
}

fn advance_tick_clock(clock: TickClock, rate: Int) -> TickClock {
  let remainder = clock.remainder + ms_per_second % rate
  TickClock(
    deadline: clock.deadline + ms_per_second / rate + remainder / rate,
    remainder: remainder % rate,
  )
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
  event: key.Event,
  key_handler: Option(OnKey(a)),
  pad_handler: Option(OnPad(a)),
) -> #(MultiWorld(a), Bool) {
  case pad.from_key(event), pad_handler {
    Some(input), Some(handler) ->
      apply_player_handler(world, player, input, Some(handler))
    _, _ -> apply_player_handler(world, player, event, key_handler)
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
      player_input(players, player, key.from_string(key), KeyPress)
    ["keydown", player, key, ..] ->
      player_input(players, player, key.from_string(key), KeyDown)
    ["keyup", player, key, ..] ->
      player_input(players, player, key.from_string(key), KeyUp)
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
  event: fn(Player, pad.Event) -> RuntimeEvent,
) -> RuntimeEvent {
  case pad.from_string(input) {
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

@external(javascript, "../sgleam/sgleam_ffi.mjs", "open_mworld")
fn open_mworld(
  width: Int,
  height: Int,
  min_players: Int,
  max_players: Int,
) -> List(String)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "mworld_uses_session")
fn mworld_uses_session() -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_player_image")
fn show_player_image(player: Int, image: Image) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "wait_mworld_event")
fn wait_mworld_event(ms: Int) -> List(String)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "close_display")
fn close_display() -> Nil
