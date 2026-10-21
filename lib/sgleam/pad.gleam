import gleam/option.{type Option, None, Some}
import sgleam/key

/// An event from the six-button teaching-game pad.
pub type Event {
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

@internal
pub fn from_string(name: String) -> Option(Event) {
  case name {
    "Up" -> Some(Up)
    "Down" -> Some(Down)
    "Left" -> Some(Left)
    "Right" -> Some(Right)
    "ActionA" -> Some(ActionA)
    "ActionB" -> Some(ActionB)
    _ -> None
  }
}

/// Maps the arrows and W, A, S, D to the directions, Z and K to `ActionA`,
/// and X and L to `ActionB`. Returns `None` for every other key.
pub fn from_key(event: key.Event) -> Option(Event) {
  case event {
    key.ArrowUp | key.Char("w") | key.Char("W") -> Some(Up)
    key.ArrowDown | key.Char("s") | key.Char("S") -> Some(Down)
    key.ArrowLeft | key.Char("a") | key.Char("A") -> Some(Left)
    key.ArrowRight | key.Char("d") | key.Char("D") -> Some(Right)
    key.Char("z") | key.Char("Z") | key.Char("k") | key.Char("K") ->
      Some(ActionA)
    key.Char("x") | key.Char("X") | key.Char("l") | key.Char("L") ->
      Some(ActionB)
    _ -> None
  }
}
