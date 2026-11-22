// Encerra ao chegar a 5 ou ao clicar em Parar; mantém a última view.
import gleam/int
import sgleam/show/button
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/simple_app
import sgleam/show/text

type Model {
  Model(count: Int, stopped: Bool)
}

type Msg {
  Increment
  Stop
}

fn init() -> Model {
  Model(0, False)
}

fn finished(model: Model) -> Bool {
  model.stopped || model.count >= 5
}

fn update(model: Model, message: Msg) -> Model {
  case message {
    Increment -> Model(..model, count: model.count + 1)
    Stop -> Model(..model, stopped: True)
  }
}

fn view(model: Model) -> Element(Msg) {
  layout.column([
    text.new("Contador: " <> int.to_string(model.count)) |> text.done,
    text.new(case finished(model) {
      True -> "Aplicação encerrada. Execute main() para iniciar novamente."
      False -> "Conte até 5 ou clique em Parar."
    })
      |> text.done,
    layout.row([
      button.new("+1")
        |> button.on_press(Increment)
        |> button.enabled(!finished(model))
        |> button.done,
      button.new("Parar")
        |> button.on_press(Stop)
        |> button.enabled(!finished(model))
        |> button.done,
    ])
      |> layout.done,
  ])
  |> layout.done
}

pub fn main() {
  simple_app.create(init, view, update)
  |> simple_app.stop_when(finished)
  |> simple_app.run
}
