//// What the library asks of the page. Only the web version has a page, so a
//// native run stops at the first `show_view`.

import gleam/dynamic.{type Dynamic}

/// Puts a frame on the page and returns its number. An event carries the
/// number of the frame it came from.
@external(javascript, "../../sgleam_ffi.mjs", "show_view")
pub fn show_view(html: String) -> Int

/// Takes the next event off the page's queue.
@external(javascript, "../../sgleam_ffi.mjs", "next_view_event")
pub fn next_event() -> Result(Dynamic, Nil)

/// Holds the program until an event arrives or `timeout_ms` passes. A negative
/// timeout waits for an event. Stop ends the program here.
@external(javascript, "../../sgleam_ffi.mjs", "wait_view_event")
pub fn wait_event(timeout_ms: Int) -> Nil

/// Makes an HTTP request and holds the program until the answer arrives. The
/// answer is `{ok: True, status, body}` or `{ok: False, error}`, where `error`
/// is `bad_url`, `timeout` or `network`. An empty `content_type` sends no
/// body.
@external(javascript, "../../sgleam_ffi.mjs", "http_request")
pub fn http_request(
  method: String,
  url: String,
  content_type: String,
  body: String,
) -> Dynamic
