import gleam/int
import gleam/string
import sgleam/check
import sgleam/show/html
import sgleam/show/button
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/simple_app
import sgleam/show/text

pub type Msg {
  Incrementar
  Decrementar
  Reiniciar
}

fn init() -> Int {
  0
}

fn update(model: Int, message: Msg) -> Int {
  case message {
    Incrementar -> model + 1
    Decrementar -> model - 1
    Reiniciar -> 0
  }
}

fn view(model: Int) -> Element(Msg) {
  layout.column([
    text.new("Valor: " <> int.to_string(model))
      |> text.done,
    layout.row([
      button.new("+")
        |> button.on_press(Incrementar)
        |> button.done,
      button.new("-")
        |> button.on_press(Decrementar)
        |> button.done,
      button.new("Reiniciar")
        |> button.on_press(Reiniciar)
        |> button.done,
    ])
      |> layout.done,
  ])
  |> layout.done
}

pub fn main() {
  simple_app.create(init, view, update)
  |> simple_app.run
}

pub fn update_examples() {
  check.eq(update(4, Incrementar), 5)
  check.eq(update(4, Decrementar), 3)
  check.eq(update(4, Reiniciar), 0)
}

pub fn view_examples() {
  check.true(string.contains(html.to_html(view(4)), "Valor: 4"))
}
