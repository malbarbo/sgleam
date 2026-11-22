// Consulta simulada por respostas predefinidas, sem rede.
import gleam/int
import gleam/string
import sgleam/task.{type Task}
import sgleam/task/fake
import sgleam/show/app
import sgleam/show/button
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/text
import sgleam/show/text_input

type Model {
  Model(url: String, preview: String, status: String)
}

type Msg {
  ChangeUrl(String)
  Fetch
  SimulationError(String)
  GotResponse(Result(String, task.HttpError))
}

fn init() -> #(Model, Task(Msg)) {
  #(
    Model("https://malbarbo.pro.br", "", "Carregando..."),
    fetch("https://malbarbo.pro.br"),
  )
}

fn fetch(url: String) -> Task(Msg) {
  use response <- task.get(url)
  task.done(GotResponse(response))
}

fn update(model: Model, message: Msg) -> #(Model, Task(Msg)) {
  case message {
    ChangeUrl(url) -> #(Model(url, "", "URL alterada."), task.none())
    Fetch -> #(Model(..model, status: "Carregando..."), fetch(model.url))
    SimulationError(reason) -> #(Model(..model, status: reason), task.none())
    GotResponse(Ok(body)) -> #(
      Model(
        ..model,
        preview: string.slice(body, 0, 500),
        status: "Resposta simulada. Exibindo até 500 caracteres.",
      ),
      task.none(),
    )
    GotResponse(Error(reason)) -> #(
      Model(..model, preview: "", status: "Erro: " <> error_text(reason)),
      task.none(),
    )
  }
}

fn error_text(error: task.HttpError) -> String {
  case error {
    task.BadUrl(url) -> "URL inválida: " <> url
    task.Timeout -> "Tempo de espera esgotado"
    task.NetworkError -> "Falha de rede simulada"
    task.BadStatus(status) -> "Status HTTP: " <> int.to_string(status)
  }
}

fn view(model: Model) -> Element(Msg) {
  layout.column([
    text.new("Consulta HTTP") |> text.done,
    text_input.new(model.url)
      |> text_input.placeholder("https://exemplo.com")
      |> text_input.on_input(ChangeUrl)
      |> text_input.done,
    button.new("Consultar") |> button.on_press(Fetch) |> button.done,
    text.new(model.status) |> text.done,
    // O conteúdo remoto é mostrado como texto, não como HTML executável.
    text.new(model.preview) |> text.done,
  ])
  |> layout.done
}

pub fn main() {
  app.create(init, view, update)
  |> app.run_with(fn(program) {
    let report =
      fake.run_messages(program, [
        fake.GetResponse(
          "https://malbarbo.pro.br",
          Ok("Conteúdo predefinido do fake."),
        ),
        fake.GetResponse("https://erro.exemplo", Error(task.NetworkError)),
      ])
    case report.result {
      Ok(messages) -> messages
      Error(fake.UnexpectedRequest(fake.Get(url))) -> [
        SimulationError("URL desconhecida no fake: " <> url),
      ]
      Error(_) -> [SimulationError("Pedido desconhecido no fake.")]
    }
  })
}
