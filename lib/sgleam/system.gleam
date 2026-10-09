import sgleam/font.{type Font}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "sleep")
pub fn sleep(ms: Int) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "now_ms")
pub fn now_ms() -> Int

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_svg")
pub fn show_svg(svg: String) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "get_key_event")
pub fn get_key_event() -> List(String)

/// Returns #(width, height) for a piece of text in `font`.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "text_metrics")
pub fn text_metrics(text: String, font: Font) -> #(Float, Float)

/// Returns #(width, height, data_uri) for a bitmap file.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "load_bitmap")
pub fn load_bitmap(path: String) -> #(Float, Float, String)
