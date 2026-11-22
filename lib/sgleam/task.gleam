/// Errors shared by GET and POST. Successful responses contain text;
/// JSON decoding belongs to the program, so there is no BadBody decoder error.
pub type HttpError {
  BadUrl(url: String)
  Timeout
  NetworkError
  BadStatus(status: Int)
}

/// A POST body and, for text, the MIME type to send as Content-Type.
/// Constructing a body does not validate or encode JSON.
pub type Body {
  EmptyBody
  StringBody(content_type: String, content: String)
}

pub fn empty_body() -> Body {
  EmptyBody
}

pub fn string_body(content_type: String, content: String) -> Body {
  StringBody(content_type, content)
}

/// Pure descriptions interpreted by the environment, never I/O functions.
pub type Task(a) {
  None
  Done(value: a)
  Get(url: String, next: fn(Result(String, HttpError)) -> Task(a))
  Post(url: String, body: Body, next: fn(Result(String, HttpError)) -> Task(a))
  WakeUp(delay_ms: Int, next: fn(Nil) -> Task(a))
}

/// Finishes without producing a value or a message.
pub fn none() -> Task(a) {
  None
}

/// Produces the final value (a message when used by App).
pub fn done(value: a) -> Task(a) {
  Done(value)
}

pub fn then(first: Task(a), next: fn(a) -> Task(b)) -> Task(b) {
  case first {
    None -> None
    Done(value) -> next(value)
    Get(url, continue) -> Get(url, fn(result) { then(continue(result), next) })
    Post(url, body, continue) ->
      Post(url, body, fn(result) { then(continue(result), next) })
    WakeUp(delay_ms, continue) ->
      WakeUp(delay_ms, fn(result) { then(continue(result), next) })
  }
}

/// Describes a GET expecting the response body as text.
pub fn get(
  url: String,
  next: fn(Result(String, HttpError)) -> Task(a),
) -> Task(a) {
  Get(url, next)
}

/// Describes a POST expecting the response body as text.
pub fn post(
  url: String,
  body: Body,
  next: fn(Result(String, HttpError)) -> Task(a),
) -> Task(a) {
  Post(url, body, next)
}

/// Requests one wake-up after a delay in milliseconds, not a periodic timer.
/// Nonpositive delays become zero. The fake acknowledges without waiting.
pub fn wake_up(delay_ms: Int, next: fn(Nil) -> Task(a)) -> Task(a) {
  let delay_ms = case delay_ms < 0 {
    True -> 0
    False -> delay_ms
  }
  WakeUp(delay_ms, next)
}
