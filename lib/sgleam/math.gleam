import gleam/wasm/math

@external(javascript, "../sgleam/sgleam_ffi.mjs", "cos_deg")
pub fn cos_deg(angle: Float) -> Float {
  cos(angle *. pi /. 180.0)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "sin_deg")
pub fn sin_deg(angle: Float) -> Float {
  sin(angle *. pi /. 180.0)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "hypot")
pub fn hypot(a: Float, b: Float) -> Float {
  math.hypot(a, b)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "cos")
pub fn cos(angle: Float) -> Float {
  math.cos(angle)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "sin")
pub fn sin(angle: Float) -> Float {
  math.sin(angle)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "atan2")
pub fn atan2(y: Float, x: Float) -> Float {
  math.atan2(y, x)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "sqrt")
pub fn sqrt(x: Float) -> Float {
  math.sqrt(x)
}

pub const pi = 3.141592653589793
