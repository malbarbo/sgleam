import sgleam/show/app
import sgleam/show/element.{type Element}
import sgleam/show/internal/application

/// Wraps an App while exposing model-only init/update functions.
pub opaque type SimpleApp(model, msg) {
  SimpleApp(application: app.App(model, msg))
}

/// Creates an application without running init yet.
/// Internally, init/update are adapted to return #(model, task.none()).
pub fn create(
  init: fn() -> model,
  view: fn(model) -> Element(msg),
  update: fn(model, msg) -> model,
) -> SimpleApp(model, msg) {
  SimpleApp(application.create_simple(init, view, update))
}

/// Stops after rendering the final model, without consuming further events.
pub fn stop_when(
  simple: SimpleApp(model, msg),
  predicate: fn(model) -> Bool,
) -> SimpleApp(model, msg) {
  SimpleApp(app.stop_when(simple.application, predicate))
}

/// Runs the wrapped App, preserving SimpleApp's event batching.
pub fn run(simple: SimpleApp(model, msg)) -> Nil {
  app.run(simple.application)
}
