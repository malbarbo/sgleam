import gleam/dynamic.{type Dynamic}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "sleep")
pub fn sleep(ms: Int) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "now_ms")
pub fn now_ms() -> Int

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_svg")
pub fn show_svg(svg: String) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_view")
pub fn show_view(html: String) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "next_event")
pub fn next_event() -> Result(Dynamic, Nil)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "wait_event")
fn wait_event_raw(timeout_ms: Int) -> Int

pub type Wait {
  HasEvent
  Timeout
  Stopped
}

pub fn wait_event(timeout_ms: Int) -> Wait {
  case wait_event_raw(timeout_ms) {
    1 -> HasEvent
    0 -> Timeout
    _ -> Stopped
  }
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "get_key_event")
pub fn get_key_event() -> List(String)

/// Returns #(width, height, x_offset, y_offset) for a piece of text. The css is
/// the whole font shorthand, size and all, and the host measures with it as it
/// comes.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "text_metrics")
pub fn text_metrics(
  text: String,
  font_css: String,
) -> #(Float, Float, Float, Float)

/// Returns #(width, height, data_uri) for a bitmap file.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "load_bitmap")
pub fn load_bitmap(path: String) -> #(Float, Float, String)
