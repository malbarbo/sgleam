@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_equal")
pub fn eq(a: x, b: x) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_approx")
pub fn approx(a: Float, b: Float, tolerance: Float) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_true")
pub fn true(val: Bool) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_false")
pub fn false(val: Bool) -> Bool
