// Envio de anotação por POST simulado; nenhum pedido de rede é realizado.
import gleam/int
import sgleam/task.{type Task}
import sgleam/task/fake
import sgleam/show/app
import sgleam/show/button
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/text
import sgleam/show/text_area
import sgleam/show/text_input

type Model {
  Model(url: String, content: String, status: String)
}

type Msg {
  ChangeUrl(String)
  Edit(String)
  Save
  SimulationError(String)
  Saved(url: String, result: Result(String, task.HttpError))
}

fn init() -> #(Model, Task(Msg)) {
  #(
    Model(
      "https://example.com/notes",
      "Minha anotação",
      "Simulação: nenhum POST será enviado.",
    ),
    task.none(),
  )
}

fn save(url: String, content: String) -> Task(Msg) {
  use result <- task.post(url, task.string_body("text/plain", content))
  task.done(Saved(url, result))
}

fn update(model: Model, message: Msg) -> #(Model, Task(Msg)) {
  case message {
    ChangeUrl(url) -> #(
      Model(..model, url:, status: "Destino alterado."),
      task.none(),
    )
    Edit(content) -> #(
      Model(..model, content:, status: "Alterações não salvas."),
      task.none(),
    )
    Save -> #(
      Model(..model, status: "Enviando..."),
      save(model.url, model.content),
    )
    SimulationError(reason) -> #(Model(..model, status: reason), task.none())
    Saved(url, result) -> {
      let status = case result {
        Ok(_) -> "Envio simulado para " <> url
        Error(reason) -> error_text(reason)
      }
      #(Model(..model, status:), task.none())
    }
  }
}

fn error_text(error: task.HttpError) -> String {
  case error {
    task.BadUrl(url) -> "URL inválida: " <> url
    task.Timeout -> "Tempo de espera esgotado"
    task.NetworkError -> "Falha de rede"
    task.BadStatus(status) -> "Status HTTP: " <> int.to_string(status)
  }
}

fn view(model: Model) -> Element(Msg) {
  layout.column([
    text.new("Bloco de notas") |> text.done,
    text_input.new(model.url)
      |> text_input.placeholder("URL de destino")
      |> text_input.on_input(ChangeUrl)
      |> text_input.done,
    text_area.new(model.content)
      |> text_area.rows(8)
      |> text_area.placeholder("Digite sua anotação")
      |> text_area.on_input(Edit)
      |> text_area.done,
    button.new("Enviar") |> button.on_press(Save) |> button.done,
    text.new(model.status) |> text.done,
  ])
  |> layout.done
}

pub fn main() {
  app.create(init, view, update)
  |> app.run_with(fn(program) {
    let report =
      fake.run_messages(program, [
        fake.PostResponse(
          "https://example.com/notes",
          task.string_body("text/plain", "Minha anotação"),
          fake.http_response(201, "recebido"),
        ),
        fake.PostResponse(
          "https://example.com/notes",
          task.string_body("text/plain", "erro"),
          fake.http_response(500, "falha"),
        ),
      ])
    case report.result {
      Ok(messages) -> messages
      Error(_) -> [
        SimulationError(
          "Pedido desconhecido: use a URL inicial e Minha anotação ou erro.",
        ),
      ]
    }
  })
}
