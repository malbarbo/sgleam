import gleam/dict.{type Dict}
import gleam/dynamic/decode
import gleam/int
import gleam/list
import gleam/result

import sgleam/show/element.{type Element}
import sgleam/show/event.{type Handler}
import sgleam/show/html
import sgleam/show/internal/host
import sgleam/show/internal/incoming
import sgleam/system
import sgleam/task.{type HttpError, type Task}

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
  let application =
    create(fn() { #(init(), task.none()) }, view, fn(model, message) {
      #(update(model, message), task.none())
    })
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
  start(app, perform)
}

/// Supplies a synchronous interpreter returning messages, never a model.
/// none() produces no messages; done(message) produces one.
pub fn run_with(
  app: App(model, msg),
  execute: fn(Task(msg)) -> List(msg),
) -> Nil {
  start(app, fn(task) { #(execute(task), []) })
}

/// What an interpreter gives back for a task: the messages it produced and
/// the wake-ups it left, each as a delay and the rest of the task.
type Outcome(msg) =
  #(List(msg), List(#(Int, fn(Nil) -> Task(msg))))

/// A wake-up that is waiting, with the time it is due in `system.elapsed_ms`.
type Wake(msg) {
  Wake(due: Int, next: fn(Nil) -> Task(msg))
}

/// The numbers and the handlers of the last frames, the newest first. An
/// event drawn on a frame that the page replaced while the event was on its
/// way still runs the handler it meant.
type Frame(msg) {
  Frame(recent: List(#(Int, Dict(Int, Handler(msg)))))
}

/// How many frames `Frame` keeps.
const kept_frames = 256

fn handlers_of(
  frame: Frame(msg),
) -> fn(Int) -> Result(Dict(Int, Handler(msg)), Nil) {
  fn(number) { list.key_find(frame.recent, number) }
}

fn start(app: App(model, msg), execute: fn(Task(msg)) -> Outcome(msg)) -> Nil {
  let #(model, task) = app.init()
  run_loop(app, execute, Draw(model, Frame([]), task, [], []))
}

/// What the loop does next. Each phase gives the next one back to
/// `run_loop`, which calls only itself, so an app that runs for a long time
/// does not grow the stack.
type Phase(model, msg) {
  /// Draws the model, then runs `task` and the messages in `pending`.
  Draw(
    model: model,
    frame: Frame(msg),
    task: Task(msg),
    pending: List(msg),
    wakes: List(Wake(msg)),
  )
  /// Runs `task` against the model on screen, then the messages it produced.
  Step(
    model: model,
    frame: Frame(msg),
    task: Task(msg),
    pending: List(msg),
    wakes: List(Wake(msg)),
  )
  /// Waits for the next wake-up or event.
  Idle(model: model, frame: Frame(msg), wakes: List(Wake(msg)))
  Stopped
}

fn run_loop(
  app: App(model, msg),
  execute: fn(Task(msg)) -> Outcome(msg),
  phase: Phase(model, msg),
) -> Nil {
  case phase {
    Stopped -> Nil
    Draw(model, frame, task, pending, wakes) ->
      run_loop(app, execute, draw(app, frame, model, task, pending, wakes))
    Step(model, frame, task, pending, wakes) ->
      run_loop(
        app,
        execute,
        step(app, execute, model, frame, task, pending, wakes),
      )
    Idle(model, frame, wakes) ->
      run_loop(app, execute, idle(app, model, frame, wakes))
  }
}

fn draw(
  app: App(model, msg),
  frame: Frame(msg),
  model: model,
  task: Task(msg),
  pending: List(msg),
  wakes: List(Wake(msg)),
) -> Phase(model, msg) {
  let #(markup, handlers) = app.view(model) |> html.render
  let frame =
    Frame([
      #(host.show_view(markup), handlers),
      ..list.take(frame.recent, kept_frames - 1)
    ])
  Step(model, frame, task, pending, wakes)
}

fn step(
  app: App(model, msg),
  execute: fn(Task(msg)) -> Outcome(msg),
  model: model,
  frame: Frame(msg),
  task: Task(msg),
  pending: List(msg),
  wakes: List(Wake(msg)),
) -> Phase(model, msg) {
  case app.should_stop(model) {
    True -> Stopped
    False -> {
      let #(messages, delays) = execute(task)
      let now = system.elapsed_ms()
      let wakes =
        list.append(
          wakes,
          list.map(delays, fn(delay) { Wake(now + delay.0, delay.1) }),
        )
      case list.append(pending, messages) {
        [message, ..rest] -> {
          let #(model, task) = app.update(model, message)
          Draw(model, frame, task, rest, wakes)
        }
        [] -> Idle(model, frame, wakes)
      }
    }
  }
}

/// A wake-up that is due runs first, so a steady stream of events does not
/// hold a timer back.
fn idle(
  app: App(model, msg),
  model: model,
  frame: Frame(msg),
  wakes: List(Wake(msg)),
) -> Phase(model, msg) {
  let now = system.elapsed_ms()
  case take_due(wakes, now, []) {
    Ok(#(next, wakes)) -> Step(model, frame, next(Nil), [], wakes)
    Error(_) ->
      case host.next_event() {
        Ok(raw) ->
          case incoming.message(raw, handlers_of(frame)) {
            Ok(message) ->
              case app.mode {
                EachUpdate -> {
                  let #(model, task) = app.update(model, message)
                  Draw(model, frame, task, [], wakes)
                }
                BatchEvents -> {
                  let #(model, _) = app.update(model, message)
                  let model = drain(app, frame, model)
                  Draw(model, frame, task.none(), [], wakes)
                }
              }
            Error(_) -> Idle(model, frame, wakes)
          }
        Error(_) -> {
          host.wait_event(timeout(wakes, now))
          Idle(model, frame, wakes)
        }
      }
  }
}

/// Returns the first wake-up that is due and the others, in order.
fn take_due(
  wakes: List(Wake(msg)),
  now: Int,
  before: List(Wake(msg)),
) -> Result(#(fn(Nil) -> Task(msg), List(Wake(msg))), Nil) {
  case wakes {
    [] -> Error(Nil)
    [wake, ..rest] ->
      case wake.due <= now {
        True -> Ok(#(wake.next, list.append(list.reverse(before), rest)))
        False -> take_due(rest, now, [wake, ..before])
      }
  }
}

/// How long to wait for an event: until the next wake-up, or forever.
fn timeout(wakes: List(Wake(msg)), now: Int) -> Int {
  case list.map(wakes, fn(wake) { wake.due }) |> list.reduce(int.min) {
    Ok(due) -> int.max(due - now, 0)
    Error(_) -> -1
  }
}

// Only create_simple enables this path, so every update returns task.none().
fn drain(app: App(model, msg), frame: Frame(msg), model: model) -> model {
  case app.should_stop(model) {
    True -> model
    False ->
      case host.next_event() {
        Error(_) -> model
        Ok(raw) -> {
          let model =
            incoming.message(raw, handlers_of(frame))
            |> result.map(fn(message) { app.update(model, message).0 })
            |> result.unwrap(model)
          drain(app, frame, model)
        }
      }
  }
}

/// The interpreter of the web version. A request holds the program until the
/// answer arrives; a wake-up is left for the loop.
fn perform(program: Task(msg)) -> Outcome(msg) {
  case program {
    task.None -> #([], [])
    task.Done(message) -> #([message], [])
    task.Get(url, next) -> perform(next(request("GET", url, task.EmptyBody)))
    task.Post(url, body, next) -> perform(next(request("POST", url, body)))
    task.WakeUp(delay_ms, next) -> #([], [#(delay_ms, next)])
  }
}

fn request(
  method: String,
  url: String,
  body: task.Body,
) -> Result(String, HttpError) {
  let #(content_type, content) = case body {
    task.EmptyBody -> #("", "")
    task.StringBody(content_type, content) -> #(content_type, content)
  }
  host.http_request(method, url, content_type, content)
  |> decode.run(response_decoder(url))
  |> result.unwrap(Error(task.NetworkError))
}

fn response_decoder(url: String) -> decode.Decoder(Result(String, HttpError)) {
  use ok <- decode.field("ok", decode.bool)
  case ok {
    True -> {
      use status <- decode.field("status", decode.int)
      use body <- decode.field("body", decode.string)
      decode.success(case status >= 200 && status < 300 {
        True -> Ok(body)
        False -> Error(task.BadStatus(status))
      })
    }
    False -> {
      use error <- decode.field("error", decode.string)
      decode.success(
        Error(case error {
          "bad_url" -> task.BadUrl(url)
          "timeout" -> task.Timeout
          _ -> task.NetworkError
        }),
      )
    }
  }
}
