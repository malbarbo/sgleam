// O fake confirma 1000 ms imediatamente: 3 -> 2 -> 1 -> 0, sem espera real.
import gleam/int
import sgleam/task.{type Task}
import sgleam/task/fake
import sgleam/show/app
import sgleam/show/element.{type Element}
import sgleam/show/text

type Msg {
  Tick
}

fn tick() -> Task(Msg) {
  use _ <- task.wake_up(1000)
  task.done(Tick)
}

fn init() -> #(Int, Task(Msg)) {
  #(3, tick())
}

fn update(model: Int, _message: Msg) -> #(Int, Task(Msg)) {
  let remaining = model - 1
  case remaining > 0 {
    True -> #(remaining, tick())
    False -> #(0, task.none())
  }
}

fn view(model: Int) -> Element(Msg) {
  text.new("Contagem simulada: " <> int.to_string(model)) |> text.done
}

pub fn main() {
  app.create(init, view, update)
  |> app.stop_when(fn(model) { model == 0 })
  |> app.run_with(fn(program) {
    fake.messages(program, [fake.WakeUpResponse(1000)])
  })
}
