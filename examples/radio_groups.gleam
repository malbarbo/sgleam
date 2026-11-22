import gleam/option.{type Option, None, Some}
import sgleam/show/button
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/radio_group
import sgleam/show/simple_app
import sgleam/show/text

type Tamanho {
  Pequeno
  Medio
  Grande
}

type Model {
  Model(
    first: Option(Tamanho),
    second: Option(Tamanho),
  )
}

type Msg {
  First(Tamanho)
  Second(Tamanho)
  Clear
}

fn init() -> Model {
  Model(None, Some(Medio))
}

fn update(model: Model, message: Msg) -> Model {
  case message {
    First(value) ->
      Model(..model, first: Some(value))

    Second(value) ->
      Model(..model, second: Some(value))

    Clear ->
      Model(None, None)
  }
}

fn nome(selecao: Option(Tamanho)) -> String {
  case selecao {
    None -> "Nenhum"
    Some(Pequeno) -> "pequeno"
    Some(Medio) -> "medio"
    Some(Grande) -> "grande"
  }
}

fn tamanhos(
  selecao: Option(Tamanho),
  rotulo: String,
  mensagem: fn(Tamanho) -> Msg,
) -> Element(Msg) {
  let grupo = case selecao {
    None -> radio_group.new()
    Some(value) ->
      radio_group.new()
      |> radio_group.select(value)
  }

  grupo
  |> radio_group.label(rotulo)
  |> radio_group.option(Pequeno, "Pequeno")
  |> radio_group.option(Medio, "Médio")
  |> radio_group.option(Grande, "Grande")
  |> radio_group.on_select(mensagem)
  |> radio_group.done
}

fn view(model: Model) -> Element(Msg) {
  layout.column([
    text.new("Primeiro: " <> nome(model.first))
      |> text.done,
    tamanhos(model.first, "Primeiro tamanho", First),
    text.new("Segundo: " <> nome(model.second))
      |> text.done,
    tamanhos(model.second, "Segundo tamanho", Second),
    button.new("Limpar seleções")
      |> button.on_press(Clear)
      |> button.done,
  ])
  |> layout.done
}

pub fn main() {
  simple_app.create(init, view, update)
  |> simple_app.run
}
