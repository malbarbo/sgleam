// Wake-up simulado: o fake reconhece o intervalo, mas não espera tempo real.
import gleam/int
import sgleam/task.{type Task}
import sgleam/task/fake
import sgleam/show/app
import sgleam/show/button
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/text

type Msg {
  Increment
  Decrement
  Reset
  IncrementLater
}

fn init() -> #(Int, Task(Msg)) {
  #(0, task.none())
}

fn update(model: Int, message: Msg) -> #(Int, Task(Msg)) {
  case message {
    Reset -> #(0, task.none())
    Increment -> #(model + 1, task.none())
    Decrement -> #(model - 1, task.none())
    IncrementLater -> #(model, delayed_increment())
  }
}

fn delayed_increment() -> Task(Msg) {
  use _ <- task.wake_up(1000)
  task.done(Increment)
}

fn view(model: Int) -> Element(Msg) {
  layout.column([
    text.new("Contador: " <> int.to_string(model)) |> text.done,
    layout.row([
      button.new("-1") |> button.on_press(Decrement) |> button.done,
      button.new("+1") |> button.on_press(Increment) |> button.done,
      button.new("Zerar") |> button.on_press(Reset) |> button.done,
      button.new("+1 após 1s (simulado)")
        |> button.on_press(IncrementLater)
        |> button.done,
    ])
      |> layout.done,
  ])
  |> layout.done
}

pub fn main() {
  app.create(init, view, update)
  |> app.run_with(fn(program) {
    fake.messages(program, [fake.WakeUpResponse(1000)])
  })
}
