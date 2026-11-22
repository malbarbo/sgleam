import gleam/list
import sgleam/task.{type Body, type HttpError, type Task}

pub type Request {
  Get(url: String)
  Post(url: String, body: Body)
  WakeUp(delay_ms: Int)
}

pub type Response {
  GetResponse(url: String, result: Result(String, HttpError))
  PostResponse(url: String, body: Body, result: Result(String, HttpError))
  WakeUpResponse(delay_ms: Int)
}

pub type Failure {
  NoValue
  UnexpectedRequest(request: Request)
}

pub type Report(a) {
  Report(result: Result(a, Failure), requests: List(Request))
}

/// Interprets a task expecting exactly one final value.
/// Use run_messages for tasks that may terminate with none().
pub fn run(program: Task(a), responses: List(Response)) -> Report(a) {
  let report = run_messages(program, responses)
  let result = case report.result {
    Ok([value]) -> Ok(value)
    Ok(_) -> Error(NoValue)
    Error(reason) -> Error(reason)
  }
  Report(result, report.requests)
}

/// Interprets a task as zero or one messages, retaining the request history.
pub fn run_messages(
  program: Task(a),
  responses: List(Response),
) -> Report(List(a)) {
  loop(program, responses, [])
}

/// Adapter for App.run_with. Unknown requests are configuration errors.
pub fn messages(program: Task(a), responses: List(Response)) -> List(a) {
  case run_messages(program, responses).result {
    Ok(messages) -> messages
    Error(_) ->
      panic as "Fake executor: unexpected request; configure an exact response"
  }
}

pub fn demo_messages(program: Task(a)) -> List(a) {
  messages(program, demo_responses())
}

fn loop(
  program: Task(a),
  responses: List(Response),
  history: List(Request),
) -> Report(List(a)) {
  case program {
    task.None -> Report(Ok([]), list.reverse(history))
    task.Done(value) -> Report(Ok([value]), list.reverse(history))
    task.Get(url, next) -> {
      let request = Get(url)
      case get_response(responses, url) {
        Ok(result) -> loop(next(result), responses, [request, ..history])
        Error(_) -> unexpected(request, history)
      }
    }
    task.Post(url, body, next) -> {
      let request = Post(url, body)
      case post_response(responses, url, body) {
        Ok(result) -> loop(next(result), responses, [request, ..history])
        Error(_) -> unexpected(request, history)
      }
    }
    task.WakeUp(delay_ms, next) -> {
      let request = WakeUp(delay_ms)
      case list.contains(responses, WakeUpResponse(delay_ms)) {
        True -> loop(next(Nil), responses, [request, ..history])
        False -> unexpected(request, history)
      }
    }
  }
}

fn unexpected(request: Request, history: List(Request)) -> Report(a) {
  Report(Error(UnexpectedRequest(request)), list.reverse([request, ..history]))
}

fn get_response(
  responses: List(Response),
  url: String,
) -> Result(Result(String, HttpError), Nil) {
  case responses {
    [] -> Error(Nil)
    [GetResponse(known, result), ..rest] ->
      case known == url {
        True -> Ok(result)
        False -> get_response(rest, url)
      }
    [_, ..rest] -> get_response(rest, url)
  }
}

fn post_response(
  responses: List(Response),
  url: String,
  body: Body,
) -> Result(Result(String, HttpError), Nil) {
  case responses {
    [] -> Error(Nil)
    [PostResponse(known_url, known_body, result), ..rest] ->
      case known_url == url && known_body == body {
        True -> Ok(result)
        False -> post_response(rest, url, body)
      }
    [_, ..rest] -> post_response(rest, url, body)
  }
}

/// Builds a fixture result from a final HTTP status and a text body.
/// Like Elm's expectString, only 2xx is successful. No request is performed.
pub fn http_response(status: Int, body: String) -> Result(String, HttpError) {
  case status >= 200 && status < 300 {
    True -> Ok(body)
    False -> Error(task.BadStatus(status))
  }
}

/// Fixtures for the GET -> POST -> wake-up example; never a live fallback.
pub fn demo_responses() -> List(Response) {
  [
    GetResponse("https://example.com/text", Ok("texto de demonstração")),
    PostResponse(
      "https://example.com/notes",
      task.string_body("text/plain", "texto de demonstração"),
      http_response(201, "recebido"),
    ),
    WakeUpResponse(1000),
  ]
}

pub fn value(program: Task(a), responses: List(Response)) -> a {
  case run(program, responses).result {
    Ok(value) -> value
    Error(NoValue) ->
      panic as "Fake executor: expected a value, but task ended with none"
    Error(UnexpectedRequest(_)) ->
      panic as "Fake executor: unexpected request; configure an exact response"
  }
}

pub fn demo_value(program: Task(a)) -> a {
  value(program, demo_responses())
}
