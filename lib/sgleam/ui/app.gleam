import gleam/dict.{type Dict}
import gleam/dynamic/decode.{type Decoder}
import gleam/result

import sgleam/system
import sgleam/ui/element.{type Element}
import sgleam/ui/event.{type Event, type Handler}
import sgleam/ui/html

/// Defines an MVU application.
/// `init` creates the initial model.
/// `view` describes the interface for a model.
/// `update` produces a new model from a message.
pub opaque type App(model, msg) {
  App(
    init: fn() -> model,
    view: fn(model) -> Element(msg),
    update: fn(model, msg) -> model,
  )
}

/// Creates an application.
pub fn create(
  init: fn() -> model,
  view: fn(model) -> Element(msg),
  update: fn(model, msg) -> model,
) -> App(model, msg) {
  App(init:, view:, update:)
}

/// Starts the application and runs its event loop.
pub fn run(app: App(model, msg)) -> Nil {
  loop(app, app.init())
}

fn loop(app: App(model, msg), model: model) -> Nil {
  let #(html, handlers) =
    app.view(model)
    |> html.render

  system.show_view(html)

  case system.wait_event(-1) {
    system.HasEvent -> loop(app, drain(app, handlers, model))

    system.Timeout -> loop(app, model)

    system.Stopped -> Nil
  }
}

fn drain(
  app: App(model, msg),
  handlers: Dict(Int, Handler(msg)),
  model: model,
) -> model {
  case system.next_event() {
    Error(_) -> model

    Ok(raw) -> {
      let next_model = {
        use incoming <- result.try(
          decode.run(raw, incoming_decoder())
          |> result.replace_error(Nil),
        )

        let Ui(handler: handler_id, event: incoming_event) = incoming

        use handler <- result.try(dict.get(handlers, handler_id))

        use message <- result.try(event.apply(handler, incoming_event))

        Ok(app.update(model, message))
      }

      drain(app, handlers, result.unwrap(next_model, model))
    }
  }
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
