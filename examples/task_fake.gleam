// Asserções puras para GET, POST, erros HTTP e wake_up. Não há rede nem espera.
import gleam/int
import gleam/list
import sgleam/task.{type Task}
import sgleam/task/fake

fn program() -> Task(Int) {
  use response <- task.get("https://example.com/text")
  case response {
    Ok(content) -> {
      use posted <- task.post(
        "https://example.com/notes",
        task.string_body("text/plain", content),
      )
      case posted {
        Ok(_) -> {
          use _ <- task.wake_up(1000)
          task.done(0)
        }
        Error(_) -> task.done(2)
      }
    }
    Error(_) -> task.done(1)
  }
}

fn success() -> String {
  let assert True =
    fake.run(program(), fake.demo_responses())
    == fake.Report(Ok(0), [
      fake.Get("https://example.com/text"),
      fake.Post(
        "https://example.com/notes",
        task.string_body("text/plain", "texto de demonstração"),
      ),
      fake.WakeUp(1000),
    ])
  "OK: GET -> POST -> wake_up e histórico completo"
}

fn http_errors() -> String {
  let errors = [
    task.BadUrl("inválida"),
    task.Timeout,
    task.NetworkError,
    task.BadStatus(404),
    task.BadStatus(500),
  ]
  let _ =
    list.map(errors, fn(error) {
      let get = task.get("url", fn(result) { task.done(result) })
      let post =
        task.post("url", task.empty_body(), fn(result) { task.done(result) })
      let assert Ok(Error(received_get)) =
        fake.run(get, [fake.GetResponse("url", Error(error))]).result
      let assert Ok(Error(received_post)) =
        fake.run(post, [
          fake.PostResponse("url", task.empty_body(), Error(error)),
        ]).result
      let assert True = received_get == error && received_post == error
    })
  "OK: GET e POST preservam todas as variantes de HttpError"
}

fn error_branches() -> String {
  let assert True =
    fake.run(program(), [
      fake.GetResponse("https://example.com/text", Error(task.NetworkError)),
    ])
    == fake.Report(Ok(1), [fake.Get("https://example.com/text")])
  let body = task.string_body("text/plain", "texto")
  let assert True =
    fake.run(program(), [
      fake.GetResponse("https://example.com/text", Ok("texto")),
      fake.PostResponse(
        "https://example.com/notes",
        body,
        Error(task.BadStatus(500)),
      ),
    ])
    == fake.Report(Ok(2), [
      fake.Get("https://example.com/text"),
      fake.Post("https://example.com/notes", body),
    ])
  "OK: erros interrompem a sequência conforme os ramos do programa"
}

fn status_codes() -> String {
  let _ =
    list.map([200, 201, 204, 299], fn(status) {
      let assert True = fake.http_response(status, "") == Ok("")
    })
  let _ =
    list.map([199, 300, 301, 400, 404, 500], fn(status) {
      let assert True =
        fake.http_response(status, "corpo") == Error(task.BadStatus(status))
    })
  "OK: status final 2xx é sucesso; demais códigos são BadStatus"
}

fn unknown_get() -> String {
  let program: Task(Int) =
    task.get("unknown", fn(_) { panic as "next não deve ser chamado" })
  let assert True =
    fake.run(program, [])
    == fake.Report(Error(fake.UnexpectedRequest(fake.Get("unknown"))), [
      fake.Get("unknown"),
    ])
  "OK: GET desconhecido não chama a continuação"
}

fn exact_post() -> String {
  let body = task.string_body("application/json", "{}")
  let program: Task(Int) =
    task.post("url", body, fn(_) { panic as "POST não deve continuar" })
  let _ =
    list.map(
      [
        fake.PostResponse("outra", body, Ok("")),
        fake.PostResponse("url", task.string_body("text/plain", "{}"), Ok("")),
        fake.PostResponse(
          "url",
          task.string_body("application/json", "[]"),
          Ok(""),
        ),
        fake.PostResponse("url", task.empty_body(), Ok("")),
        fake.GetResponse("url", Ok("")),
      ],
      fn(response) {
        let request = fake.Post("url", body)
        let assert True =
          fake.run(program, [response])
          == fake.Report(Error(fake.UnexpectedRequest(request)), [request])
      },
    )
  "OK: POST exige método, URL, tipo de conteúdo e corpo correspondentes"
}

fn wake_up() -> String {
  let program: Task(Int) = {
    use _ <- task.wake_up(1000)
    task.done(42)
  }
  let assert True =
    fake.run(program, [fake.WakeUpResponse(1000)])
    == fake.Report(Ok(42), [fake.WakeUp(1000)])
  let unknown: Task(Int) =
    task.wake_up(1000, fn(_) { panic as "intervalo desconhecido" })
  let assert True =
    fake.run(unknown, [fake.WakeUpResponse(999)]).result
    == Error(fake.UnexpectedRequest(fake.WakeUp(1000)))
  let _ =
    list.map([-10, 0], fn(delay) {
      let zero = task.wake_up(delay, fn(_) { task.done(Nil) })
      let assert True =
        fake.run(zero, [fake.WakeUpResponse(0)])
        == fake.Report(Ok(Nil), [fake.WakeUp(0)])
    })
  "OK: wake_up usa milissegundos, exige confirmação e normaliza negativos"
}

fn unknown_preserves_history() -> String {
  let program: Task(Int) = {
    use _ <- task.get("url")
    use _ <- task.wake_up(10)
    panic as "Não deve chegar aqui"
  }
  let assert True =
    fake.run(program, [fake.GetResponse("url", Ok(""))])
    == fake.Report(Error(fake.UnexpectedRequest(fake.WakeUp(10))), [
      fake.Get("url"),
      fake.WakeUp(10),
    ])
  "OK: pedido desconhecido preserva o histórico anterior"
}

fn composition() -> String {
  let composed = {
    use value <- task.then(program())
    use _ <- task.wake_up(5)
    task.done(int.to_string(value + 10))
  }
  let report =
    fake.run(composed, [fake.WakeUpResponse(5), ..fake.demo_responses()])
  let assert Ok("10") = report.result
  let assert True =
    report.requests
    == [
      fake.Get("https://example.com/text"),
      fake.Post(
        "https://example.com/notes",
        task.string_body("text/plain", "texto de demonstração"),
      ),
      fake.WakeUp(1000),
      fake.WakeUp(5),
    ]
  let assert Ok("42") =
    fake.run(
      task.then(task.done(42), fn(n) { task.done(int.to_string(n)) }),
      [],
    ).result
  "OK: then preserva GET, POST e wake_up e permite mudar o tipo final"
}

fn repeatable() -> String {
  let replies = fake.demo_responses()
  let assert True = fake.run(program(), replies) == fake.run(program(), replies)
  let repeated = {
    use _ <- task.wake_up(1000)
    use _ <- task.wake_up(1000)
    task.done(Nil)
  }
  let assert True =
    fake.run(repeated, [fake.WakeUpResponse(1000)])
    == fake.Report(Ok(Nil), [fake.WakeUp(1000), fake.WakeUp(1000)])
  "OK: simulação repetível e respostas reutilizáveis"
}

fn first_response_and_adapters() -> String {
  let get = task.get("url", fn(result) { task.done(result) })
  let assert Ok(Ok("primeira")) =
    fake.run(get, [
      fake.GetResponse("url", Ok("primeira")),
      fake.GetResponse("url", Error(task.Timeout)),
    ]).result
  let post =
    task.post("url", task.empty_body(), fn(result) { task.done(result) })
  let assert Ok(Ok("primeira")) =
    fake.run(post, [
      fake.PostResponse("url", task.empty_body(), Ok("primeira")),
      fake.PostResponse("url", task.empty_body(), Error(task.Timeout)),
    ]).result
  let assert 0 = fake.value(program(), fake.demo_responses())
  let assert 0 = fake.demo_value(program())
  let assert True = fake.demo_messages(program()) == [0]
  "OK: primeira resposta correspondente e adaptadores"
}

fn no_message() -> String {
  let empty: Task(Int) = task.none()
  let assert True = fake.run_messages(empty, []) == fake.Report(Ok([]), [])
  let assert True = fake.run(empty, []).result == Error(fake.NoValue)
  let chained = task.then(empty, fn(_) { panic as "none não deve chamar next" })
  let assert True = fake.messages(chained, []) == []
  let silent: Task(Int) = {
    use _ <- task.wake_up(0)
    task.none()
  }
  let assert True =
    fake.run_messages(silent, [fake.WakeUpResponse(0)])
    == fake.Report(Ok([]), [fake.WakeUp(0)])
  let assert True = fake.messages(task.done(42), []) == [42]
  "OK: none não produz mensagem; done produz uma"
}

pub fn smain() -> List(String) {
  [
    success(),
    http_errors(),
    error_branches(),
    status_codes(),
    unknown_get(),
    exact_post(),
    wake_up(),
    unknown_preserves_history(),
    composition(),
    repeatable(),
    first_response_and_adapters(),
    no_message(),
  ]
}
