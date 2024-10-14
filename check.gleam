@external(javascript, "../sgleam_ffi.mjs", "check_equal")
pub fn eq(a: x, b: x) -> Bool

@external(javascript, "../sgleam_ffi.mjs", "check_approx")
pub fn approx(a: Float, b: Float, tolerance: Float) -> Bool

pub fn true(val: Bool) -> Bool {
  eq(val, True)
}

pub fn false(val: Bool) -> Bool {
  eq(val, False)
}
