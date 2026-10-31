import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import sgleam/env.{type Env}
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

pub type KeysAsPad =
  fn(key.Event) -> Option(pad.Event)

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
    init: fn(Env, List(Player)) -> a,
    to_image: ToImage(a),
    rate: Int,
    min_players: Int,
    max_players: Int,
    on_tick: Option(OnTick(a)),
    stop_when: Option(StopWhen(a)),
    on_key_press: Option(OnKey(a)),
    on_key_down: Option(OnKey(a)),
    on_key_up: Option(OnKey(a)),
    on_pad_down: Option(OnPad(a)),
    on_pad_up: Option(OnPad(a)),
    keys_as_pad: KeysAsPad,
  )
}

/// Creates a world whose fixed player list is chosen when the match starts.
/// `min` stays between 1 and 1024, and `max` between `min` and 1024.
pub fn create(
  state: a,
  min min_players: Int,
  max max_players: Int,
  to_image to_image: ToImage(a),
) -> MultiWorld(a) {
  create_from(fn(_, _) { state }, min: min_players, max: max_players, to_image:)
}

/// Creates a world whose state `init` makes when the match starts, from the
/// env of the run and the players of the match.
pub fn create_from(
  init: fn(Env, List(Player)) -> a,
  min min_players: Int,
  max max_players: Int,
  to_image to_image: ToImage(a),
) -> MultiWorld(a) {
  let min_players = int.clamp(min_players, 1, max_player_count)
  let max_players = int.clamp(max_players, min_players, max_player_count)
  MultiWorld(
    init:,
    to_image:,
    rate: default_tick_rate,
    min_players:,
    max_players:,
    on_tick: None,
    stop_when: None,
    on_key_press: None,
    on_key_down: None,
    on_key_up: None,
    on_pad_down: None,
    on_pad_up: None,
    keys_as_pad: fn(_) { None },
  )
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

/// Sends a key that `mapping` turns into a pad button to `on_pad_down` and
/// `on_pad_up`, and to no key handler. Every other key stays a key.
pub fn keys_as_pad(world: MultiWorld(a), mapping: KeysAsPad) -> MultiWorld(a) {
  MultiWorld(..world, keys_as_pad: mapping)
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
  let players =
    open_mworld(world.min_players, world.max_players)
    |> players_from_start
  let running =
    Running(world:, players:, state: world.init(env.from_host(), players))
  show_players(running)
  run_loop(running, start_tick_clock(system.now_ms(), world.rate), False)
  close_display()
}

/// A world that runs, with the players of the match and the current state.
type Running(a) {
  Running(world: MultiWorld(a), players: List(Player), state: a)
}

type RuntimeEvent {
  KeyPress(Player, key.Event)
  KeyDown(Player, key.Event)
  KeyUp(Player, key.Event)
  PadDown(Player, pad.Event)
  PadUp(Player, pad.Event)
  DisplayTick
  NoEvent
}

/// `changed` says if the state changed since the players last got their
/// views.
fn run_loop(running: Running(a), clock: TickClock, changed: Bool) -> Nil {
  case running.world.stop_when |> option.map(fn(stop) { stop(running.state) }) {
    // The close of the display shows the last views.
    Some(True) if changed -> show_players(running)
    Some(True) -> Nil
    _ ->
      case system.wait_event(int.max(clock.deadline - system.now_ms(), 0)) {
        ["close"] -> Nil
        event -> {
          let #(running, clock, changed) =
            turn(running, runtime_event(running.players, event), clock, changed)
          run_loop(running, clock, changed)
        }
      }
  }
}

/// Handles a logical tick when one is due and one event. A tick of the
/// display gives each player a view of the state if the state changed, and
/// the views go out at the next tick. Returns `changed` for the next turn.
fn turn(
  running: Running(a),
  event: RuntimeEvent,
  clock: TickClock,
  changed: Bool,
) -> #(Running(a), TickClock, Bool) {
  let now = system.now_ms()
  let rate = running.world.rate
  let #(running, ticked, clock) = case now >= clock.deadline {
    True -> {
      let #(running, ticked) = case running.world.on_tick {
        Some(on_tick) -> #(
          Running(..running, state: on_tick(running.state)),
          True,
        )
        None -> #(running, False)
      }
      let bumped = advance_tick_clock(clock, rate)
      let clock = case bumped.deadline <= now {
        True -> start_tick_clock(now, rate)
        False -> bumped
      }
      #(running, ticked, clock)
    }
    False -> #(running, False, clock)
  }
  let #(running, handled) = apply_event(running, event)
  case event, changed || ticked || handled {
    DisplayTick, True -> {
      show_players(running)
      #(running, clock, False)
    }
    _, changed -> #(running, clock, changed)
  }
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
  running: Running(a),
  event: RuntimeEvent,
) -> #(Running(a), Bool) {
  let world = running.world
  case event {
    KeyPress(player, key) ->
      case world.keys_as_pad(key) {
        Some(_) -> #(running, False)
        None -> apply_player_handler(running, player, key, world.on_key_press)
      }
    KeyDown(player, key) ->
      apply_key_or_pad_handler(
        running,
        player,
        key,
        world.on_key_down,
        world.on_pad_down,
      )
    KeyUp(player, key) ->
      apply_key_or_pad_handler(
        running,
        player,
        key,
        world.on_key_up,
        world.on_pad_up,
      )
    PadDown(player, input) ->
      apply_player_handler(running, player, input, world.on_pad_down)
    PadUp(player, input) ->
      apply_player_handler(running, player, input, world.on_pad_up)
    DisplayTick | NoEvent -> #(running, False)
  }
}

fn apply_key_or_pad_handler(
  running: Running(a),
  player: Player,
  event: key.Event,
  key_handler: Option(OnKey(a)),
  pad_handler: Option(OnPad(a)),
) -> #(Running(a), Bool) {
  case running.world.keys_as_pad(event) {
    Some(input) -> apply_player_handler(running, player, input, pad_handler)
    None -> apply_player_handler(running, player, event, key_handler)
  }
}

fn apply_player_handler(
  running: Running(a),
  player: Player,
  input: input,
  handler: Option(fn(a, Player, input) -> a),
) -> #(Running(a), Bool) {
  case handler {
    Some(handler) -> #(
      Running(..running, state: handler(running.state, player, input)),
      True,
    )
    None -> #(running, False)
  }
}

fn show_players(running: Running(a)) {
  list.each(running.players, fn(player) {
    running.world.to_image(running.state, player)
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
    ["tick"] -> DisplayTick
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
fn open_mworld(min_players: Int, max_players: Int) -> List(String)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_player_image")
fn show_player_image(player: Int, image: Image) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "close_display")
fn close_display() -> Nil
