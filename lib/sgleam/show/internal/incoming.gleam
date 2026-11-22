import gleam/dict.{type Dict}
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode.{type Decoder}
import gleam/result
import sgleam/show/event.{type Event, type Handler}

/// Decodes a host event and applies its registered handler.
/// Malformed events, missing handlers and incompatible values are ignored.
pub fn message(raw: Dynamic, handlers: Dict(Int, Handler(msg))) -> Result(msg, Nil) {
  use incoming <- result.try(
    decode.run(raw, incoming_decoder()) |> result.replace_error(Nil),
  )
  let Ui(handler: handler_id, event: incoming_event) = incoming
  use handler <- result.try(dict.get(handlers, handler_id))
  event.apply(handler, incoming_event)
}

type Incoming {
  Ui(handler: Int, event: Event)
}

fn incoming_decoder() -> Decoder(Incoming) {
  use kind <- decode.field("kind", decode.string)

  case kind {
    "ui" -> {
      use handler <- decode.field("h", decode.int)
      use incoming_event <- decode.then(event_decoder())

      decode.success(Ui(handler: handler, event: incoming_event))
    }

    _ ->
      decode.failure(Ui(handler: 0, event: event.NoValue), expected: "Incoming")
  }
}

fn event_decoder() -> Decoder(Event) {
  use event_kind <- decode.field("e", decode.string)

  case event_kind {
    "click" -> decode.success(event.NoValue)

    "input" | "submit" | "select" -> {
      use value <- decode.field("value", decode.string)
      decode.success(event.StringValue(value))
    }

    "check" -> {
      use checked <- decode.field("checked", decode.bool)
      decode.success(event.BoolValue(checked))
    }

    _ -> decode.failure(event.NoValue, expected: "Event")
  }
}
