// Consulta HTTP de verdade. Só funciona na versão web, e só com servidores
// que permitem CORS.
import gleam/int
import gleam/string
import sgleam/show/app
import sgleam/show/button
import sgleam/show/element.{type Element}
import sgleam/show/layout
import sgleam/show/text
import sgleam/show/text_input
import sgleam/task.{type Task}

const readme = "https://raw.githubusercontent.com/malbarbo/sgleam/main/README.md"

type Model {
  Model(url: String, preview: String, status: String)
}

type Msg {
  ChangeUrl(String)
  Fetch
  GotResponse(Result(String, task.HttpError))
}

fn init() -> #(Model, Task(Msg)) {
  #(Model(readme, "", "Carregando..."), fetch(readme))
}

fn fetch(url: String) -> Task(Msg) {
  use response <- task.get(url)
  task.done(GotResponse(response))
}

fn update(model: Model, message: Msg) -> #(Model, Task(Msg)) {
  case message {
    ChangeUrl(url) -> #(Model(url, "", "URL alterada."), task.none())
    Fetch -> #(Model(..model, status: "Carregando..."), fetch(model.url))
    GotResponse(Ok(body)) -> #(
      Model(
        ..model,
        preview: string.slice(body, 0, 500),
        status: "Exibindo até 500 caracteres.",
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
    task.NetworkError -> "Falha de rede (o servidor permite CORS?)"
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
  |> app.run
}
