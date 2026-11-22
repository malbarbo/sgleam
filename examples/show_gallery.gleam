// Galeria de todos os componentes de sgleam/show, sem tarefas ou I/O no modelo.
// No playground, carregue o arquivo e execute main() no REPL.
import gleam/int
import gleam/option.{type Option, None, Some}
import sgleam/show/button
import sgleam/show/checkbox
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/link
import sgleam/show/picker
import sgleam/show/radio_group
import sgleam/show/simple_app
import sgleam/show/text
import sgleam/show/text_area
import sgleam/show/text_input

type Model {
  Model(
    clicks: Int,
    name: String,
    notes: String,
    checked: Bool,
    size: Option(String),
    color: String,
    last_event: String,
  )
}

type Msg {
  Click
  Reset
  NameChanged(String)
  NameSubmitted(String)
  NotesChanged(String)
  Checked(Bool)
  SizeSelected(String)
  ColorSelected(String)
  LinkPressed
}

fn init() -> Model {
  Model(0, "", "", False, None, "azul", "Nenhuma interação ainda.")
}

fn update(model: Model, message: Msg) -> Model {
  case message {
    Click ->
      Model(..model, clicks: model.clicks + 1, last_event: "Botão clicado.")
    Reset -> init()
    NameChanged(name) -> Model(..model, name:, last_event: "Texto editado.")
    NameSubmitted(name) ->
      Model(..model, name:, last_event: "Enter enviado: " <> name)
    NotesChanged(notes) ->
      Model(..model, notes:, last_event: "Área de texto editada.")
    Checked(checked) ->
      Model(..model, checked:, last_event: "Checkbox alterado.")
    SizeSelected(size) ->
      Model(
        ..model,
        size: Some(size),
        last_event: "Radio selecionado: " <> size,
      )
    ColorSelected(color) ->
      Model(..model, color:, last_event: "Picker selecionado: " <> color)
    LinkPressed -> Model(..model, last_event: "Link com evento clicado.")
  }
}

fn label(value: String) -> Element(Msg) {
  text.new(value) |> text.done
}

fn section(title: String, children: List(Element(Msg))) -> Element(Msg) {
  layout.column([label(title), ..children]) |> layout.done
}

fn colors() -> List(element.Choice) {
  [
    picker.choice("Azul", "azul"),
    picker.choice("Verde", "verde"),
    picker.choice("Vermelho — opção com texto maior", "vermelho"),
  ]
}

fn view(model: Model) -> Element(Msg) {
  layout.column([
    label("GALERIA DE COMPONENTES — SimpleApp"),
    label(
      "Teste tamanhos, alinhamento, foco, digitação e estados desabilitados.",
    ),
    section("1. Text", [
      label("Texto curto"),
      label(
        "Texto mais longo para observar quebra de linha quando a área do playground fica estreita. Acentos: ação, coração, café.",
      ),
      label("Caracteres literais: <tag> & aspas \"exemplo\""),
    ]),
    section("2. Button — compare a largura com o texto", [
      layout.row([
        button.new("OK") |> button.on_press(Click) |> button.done,
        button.new("Botão com texto mais comprido")
          |> button.on_press(Click)
          |> button.done,
        button.new("Desabilitado")
          |> button.on_press(Click)
          |> button.enabled(False)
          |> button.done,
      ])
        |> layout.done,
      label("Cliques: " <> int.to_string(model.clicks)),
      // A column também preserva a largura natural do botão.
      button.new("Botão em coluna") |> button.on_press(Click) |> button.done,
    ]),
    section("3. TextInput", [
      label("Nome (pressione Enter para testar on_submit):"),
      text_input.new(model.name)
        |> text_input.placeholder("Digite seu nome")
        |> text_input.on_input(NameChanged)
        |> text_input.on_submit(NameSubmitted)
        |> text_input.done,
      text_input.new("Entrada desabilitada")
        |> text_input.enabled(False)
        |> text_input.done,
      label("Valor: " <> model.name),
    ]),
    section("4. TextArea", [
      label("Observações (quatro linhas):"),
      text_area.new(model.notes)
        |> text_area.placeholder("Digite várias linhas...")
        |> text_area.rows(4)
        |> text_area.on_input(NotesChanged)
        |> text_area.done,
      text_area.new("Área desabilitada")
        |> text_area.rows(2)
        |> text_area.enabled(False)
        |> text_area.done,
    ]),
    section("5. Checkbox", [
      layout.row([
        checkbox.new(model.checked)
          |> checkbox.on_change(Checked)
          |> checkbox.done,
        label("Opção interativa"),
        checkbox.new(True) |> checkbox.enabled(False) |> checkbox.done,
        label("Marcado e desabilitado"),
        checkbox.new(False) |> checkbox.enabled(False) |> checkbox.done,
        label("Desmarcado e desabilitado"),
      ])
        |> layout.done,
      label(case model.checked {
        True -> "Estado: marcado"
        False -> "Estado: desmarcado"
      }),
    ]),
    section("6. RadioGroup — seleção inicialmente vazia", [
      {
        case model.size {
          None -> radio_group.new()
          Some(value) -> radio_group.new() |> radio_group.select(value)
        }
      }
        |> radio_group.label("Tamanho")
        |> radio_group.option("pequeno", "Pequeno")
        |> radio_group.option("medio", "Médio")
        |> radio_group.option("grande", "Grande")
        |> radio_group.on_select(SizeSelected)
        |> radio_group.done,
      radio_group.new()
        |> radio_group.select("fixo")
        |> radio_group.option("fixo", "Radio desabilitado")
        |> radio_group.enabled(False)
        |> radio_group.done,
      label("Selecionado: " <> option.unwrap(model.size, "Nenhum")),
    ]),
    section("7. Picker (seleção)", [
      // Tanto row quanto column preservam aqui a largura natural do picker.
      layout.row([
        picker.new(colors())
        |> picker.selected(model.color)
        |> picker.on_select(ColorSelected)
        |> picker.done,
      ])
        |> layout.done,
      layout.row([
        picker.new(colors())
        |> picker.selected("verde")
        |> picker.enabled(False)
        |> picker.done,
      ])
        |> layout.done,
      label("Cor selecionada: " <> model.color),
    ]),
    section("8. Link", [
      link.new("Link com evento (veja o status abaixo)", "#gallery")
        |> link.on_press(LinkPressed)
        |> link.done,
      link.new("Site do Gleam — nova aba", "https://gleam.run")
        |> link.new_tab(True)
        |> link.done,
    ]),
    section("9. Layout — linhas e colunas aninhadas", [
      layout.row([
        layout.column([label("Coluna A"), label("A1"), label("A2")])
          |> layout.done,
        layout.column([label("Coluna B"), label("B1"), label("B2")])
          |> layout.done,
      ])
      |> layout.done,
    ]),
    section("Status", [
      label(model.last_event),
      button.new("Restaurar valores iniciais")
        |> button.on_press(Reset)
        |> button.done,
    ]),
  ])
  |> layout.done
}

pub fn main() {
  simple_app.create(init, view, update) |> simple_app.run
}
