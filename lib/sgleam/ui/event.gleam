/// A value produced by an interaction in the host/browser.
pub type Event {
  NoValue
  StringValue(String)
  BoolValue(Bool)
}

/// A typed handler associated with an interactive element.
/// The handler stays inside the Gleam runtime. Only its numeric identifier
/// crosses the boundary to the host/browser.
pub opaque type Handler(msg) {
  Constant(msg)
  FromString(fn(String) -> msg)
  FromBool(fn(Bool) -> msg)
}

/// Creates a handler whose message is already known.
/// Used by interactions such as button presses.
pub fn constant(message: msg) -> Handler(msg) {
  Constant(message)
}

/// Creates a handler whose message depends on a String produced by the host.
/// Used by components such as text inputs and selects.
pub fn from_string(to_message: fn(String) -> msg) -> Handler(msg) {
  FromString(to_message)
}

/// Creates a handler whose message depends on a Bool produced by the host.
/// Used by components such as checkboxes.
pub fn from_bool(to_message: fn(Bool) -> msg) -> Handler(msg) {
  FromBool(to_message)
}

/// Applies an incoming host event to its corresponding handler.
/// Incompatible handler/event pairs return Error instead of crashing the
/// application.
pub fn apply(handler: Handler(msg), event: Event) -> Result(msg, Nil) {
  case handler, event {
    Constant(message), NoValue -> Ok(message)

    FromString(to_message), StringValue(value) -> Ok(to_message(value))

    FromBool(to_message), BoolValue(value) -> Ok(to_message(value))

    _, _ -> Error(Nil)
  }
}

/// Transforms the messages produced by a handler.
pub fn map(handler: Handler(a), transform: fn(a) -> b) -> Handler(b) {
  case handler {
    Constant(message) -> Constant(transform(message))

    FromString(to_message) ->
      FromString(fn(value) { transform(to_message(value)) })

    FromBool(to_message) -> FromBool(fn(value) { transform(to_message(value)) })
  }
}
