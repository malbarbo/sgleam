import sgleam/task.{type Task}
import sgleam/show/element.{type Element}
import sgleam/show/internal/application

/// An MVU application whose init/update return a model and a task.
pub type App(model, msg) =
  application.App(model, msg)

pub fn create(
  init: fn() -> #(model, Task(msg)),
  view: fn(model) -> Element(msg),
  update: fn(model, msg) -> #(model, Task(msg)),
) -> App(model, msg) {
  application.create(init, view, update)
}

/// Renders the final model and stops before its task or queued messages/events.
pub fn stop_when(
  app: App(model, msg),
  predicate: fn(model) -> Bool,
) -> App(model, msg) {
  application.stop_when(app, predicate)
}

pub fn run(app: App(model, msg)) -> Nil {
  application.run(app)
}

/// Supplies a synchronous interpreter returning messages, never a model.
pub fn run_with(
  app: App(model, msg),
  execute: fn(Task(msg)) -> List(msg),
) -> Nil {
  application.run_with(app, execute)
}
