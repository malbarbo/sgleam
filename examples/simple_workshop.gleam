// Formulário local: todos os componentes, sem operações externas.
// No playground, carregue o arquivo e execute main().
import gleam/string
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

type Modality {
  InPerson
  Remote
}

type Model {
  Model(
    name: String,
    expectations: String,
    materials: Bool,
    modality: Modality,
    workshop: String,
    confirmed: Bool,
    show_schedule: Bool,
  )
}

type Msg {
  NameChanged(String)
  ExpectationsChanged(String)
  MaterialsChanged(Bool)
  ModalityChanged(Modality)
  WorkshopChanged(String)
  Confirm
  Reset
  ShowSchedule
}

fn init() -> Model {
  Model("", "", False, InPerson, "Introdução ao Gleam", False, False)
}

fn update(model: Model, message: Msg) -> Model {
  case message {
    NameChanged(name) -> Model(..model, name:, confirmed: False)
    ExpectationsChanged(expectations) ->
      Model(..model, expectations:, confirmed: False)
    MaterialsChanged(materials) -> Model(..model, materials:, confirmed: False)
    ModalityChanged(modality) -> Model(..model, modality:, confirmed: False)
    WorkshopChanged(workshop) -> Model(..model, workshop:, confirmed: False)
    Confirm -> Model(..model, confirmed: string.trim(model.name) != "")
    Reset -> init()
    ShowSchedule -> Model(..model, show_schedule: !model.show_schedule)
  }
}

fn label(value: String) -> Element(Msg) {
  text.new(value) |> text.done
}

fn modality_name(modality: Modality) -> String {
  case modality {
    InPerson -> "Presencial"
    Remote -> "Remota"
  }
}

fn view(model: Model) -> Element(Msg) {
  layout.column([
    label("SHOW — Inscrição em uma oficina"),
    label("Preencha os dados e confirme para visualizar sua inscrição."),
    label("Nome:"),
    text_input.new(model.name)
      |> text_input.placeholder("Seu nome")
      |> text_input.on_input(NameChanged)
      |> text_input.done,
    label("Oficina:"),
    picker.new([
      picker.choice("Introdução ao Gleam", "Introdução ao Gleam"),
      picker.choice("Interfaces com SHOW", "Interfaces com SHOW"),
      picker.choice("Modelando mensagens", "Modelando mensagens"),
    ])
      |> picker.selected(model.workshop)
      |> picker.on_select(WorkshopChanged)
      |> picker.done,
    radio_group.new()
      |> radio_group.label("Modalidade")
      |> radio_group.select(model.modality)
      |> radio_group.option(InPerson, "Presencial")
      |> radio_group.option(Remote, "Remota")
      |> radio_group.on_select(ModalityChanged)
      |> radio_group.done,
    label("O que você espera aprender?"),
    text_area.new(model.expectations)
      |> text_area.placeholder("Conte suas expectativas...")
      |> text_area.rows(3)
      |> text_area.on_input(ExpectationsChanged)
      |> text_area.done,
    layout.row([
      checkbox.new(model.materials)
        |> checkbox.on_change(MaterialsChanged)
        |> checkbox.done,
      label("Quero receber material complementar"),
    ])
      |> layout.done,
    link.new("Ver programação das oficinas", "#programacao")
      |> link.on_press(ShowSchedule)
      |> link.done,
    case model.show_schedule {
      True ->
        label(
          "Programação ilustrativa: Gleam às 9h; SHOW às 11h; mensagens às 14h.",
        )
      False -> label("")
    },
    layout.row([
      button.new("Confirmar inscrição")
        |> button.enabled(string.trim(model.name) != "")
        |> button.on_press(Confirm)
        |> button.done,
      button.new("Limpar") |> button.on_press(Reset) |> button.done,
    ])
      |> layout.done,
    case model.confirmed {
      False ->
        label(
          "A confirmação será exibida aqui. Informe seu nome para continuar.",
        )
      True ->
        layout.column([
          label("Inscrição confirmada nesta demonstração"),
          label("Participante: " <> string.trim(model.name)),
          label("Oficina: " <> model.workshop),
          label("Modalidade: " <> modality_name(model.modality)),
          label("Expectativas: " <> model.expectations),
          label(case model.materials {
            True -> "Material complementar: sim"
            False -> "Material complementar: não"
          }),
          label(
            "Os dados ficam apenas na aplicação; não são enviados nem salvos.",
          ),
        ])
        |> layout.done
    },
  ])
  |> layout.done
}

pub fn main() {
  simple_app.create(init, view, update) |> simple_app.run
}
