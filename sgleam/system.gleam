@external(javascript, "../sgleam/sgleam_ffi.mjs", "sleep")
pub fn sleep(ms: Int) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "show_svg")
pub fn show_svg(svg: String) -> Nil

@external(javascript, "../sgleam/sgleam_ffi.mjs", "get_key_event")
pub fn get_key_event() -> List(String)
