import gleam/dict.{type Dict}
import gleam/list
import gleam/result

import sgleam/system
import sgleam/task.{type Task}
import sgleam/task/fake
import sgleam/show/element.{type Element}
import sgleam/show/event.{type Handler}
import sgleam/show/internal/incoming
import sgleam/show/html

/// Pure init/update return the current model and a task producing messages.
/// The environment renders the model, interprets the task, and routes each
/// returned message through update. The default interpreter is synchronous.
pub opaque type App(model, msg) {
  App(
    init: fn() -> #(model, Task(msg)),
    view: fn(model) -> Element(msg),
    update: fn(model, msg) -> #(model, Task(msg)),
    should_stop: fn(model) -> Bool,
    mode: Mode,
  )
}

pub fn create(
  init: fn() -> #(model, Task(msg)),
  view: fn(model) -> Element(msg),
  update: fn(model, msg) -> #(model, Task(msg)),
) -> App(model, msg) {
  App(init:, view:, update:, should_stop: fn(_) { False }, mode: EachUpdate)
}

type Mode {
  EachUpdate
  BatchEvents
}

/// Internal constructor: only model-only callbacks can enable batching.
pub fn create_simple(
  init: fn() -> model,
  view: fn(model) -> Element(msg),
  update: fn(model, msg) -> model,
) -> App(model, msg) {
  let application = create(
    fn() { #(init(), task.none()) },
    view,
    fn(model, message) { #(update(model, message), task.none()) },
  )
  App(..application, mode: BatchEvents)
}

/// Renders the final model and stops before its task or queued messages/events.
pub fn stop_when(
  app: App(model, msg),
  predicate: fn(model) -> Bool,
) -> App(model, msg) {
  App(..app, should_stop: predicate)
}

pub fn run(app: App(model, msg)) -> Nil {
  case app.mode {
    EachUpdate -> run_with(app, fake.demo_messages)
    BatchEvents -> run_with(app, fn(_) { [] })
  }
}

/// Supplies a synchronous interpreter returning messages, never a model.
/// none() produces no messages; done(message) produces one.
pub fn run_with(
  app: App(model, msg),
  execute: fn(Task(msg)) -> List(msg),
) -> Nil {
  loop(app, app.init(), [], execute)
}

fn loop(
  app: App(model, msg),
  state: #(model, Task(msg)),
  pending: List(msg),
  execute: fn(Task(msg)) -> List(msg),
) -> Nil {
  let #(model, task) = state
  let #(markup, handlers) = app.view(model) |> html.render
  system.show_view(markup)

  case app.should_stop(model) {
    True -> Nil
    False -> {
      let messages = list.append(pending, execute(task))
      case messages {
        [message, ..rest] ->
          loop(app, app.update(model, message), rest, execute)
        [] ->
          case app.mode {
            EachUpdate -> await_event(app, handlers, model, execute)
            BatchEvents ->
              case system.wait_event(-1) {
                system.HasEvent ->
                  loop(app, #(drain(app, handlers, model), task.none()), [], execute)
                system.Timeout -> loop(app, #(model, task.none()), [], execute)
                system.Stopped -> Nil
              }
          }
      }
    }
  }
}

fn await_event(
  app: App(model, msg),
  handlers: Dict(Int, Handler(msg)),
  model: model,
  execute: fn(Task(msg)) -> List(msg),
) -> Nil {
  case system.next_event() {
    Ok(raw) -> {
      case incoming.message(raw, handlers) {
        Ok(message) -> loop(app, app.update(model, message), [], execute)
        Error(_) -> await_event(app, handlers, model, execute)
      }
    }
    Error(_) ->
      case system.wait_event(-1) {
        system.Stopped -> Nil
        system.HasEvent | system.Timeout ->
          await_event(app, handlers, model, execute)
      }
  }
}

// Only create_simple enables this path, so every update returns task.none().
fn drain(
  app: App(model, msg),
  handlers: Dict(Int, Handler(msg)),
  model: model,
) -> model {
  case app.should_stop(model) {
    True -> model
    False ->
      case system.next_event() {
        Error(_) -> model
        Ok(raw) -> {
          let next_model =
            incoming.message(raw, handlers)
            |> result.map(fn(message) {
              let #(updated, _) = app.update(model, message)
              updated
            })
            |> result.unwrap(model)
          drain(app, handlers, next_model)
        }
      }
  }
}
