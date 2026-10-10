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

/// An image that the engine holds, which a bitmap draws.
pub type Loaded

/// Returns #(width, height, image) for a bitmap file. A file that does not
/// load throws an error.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "load_bitmap")
pub fn load_bitmap(path: String) -> #(Float, Float, Loaded)

/// Returns #(width, height, image) for the image in `data_uri`, or an error
/// when the data URI holds no image.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "load_image")
pub fn load_image(data_uri: String) -> Result(#(Float, Float, Loaded), Nil)

/// An image that draws nothing, for a bitmap that only stands for its class.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "no_image")
pub fn no_image() -> Loaded
