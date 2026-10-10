import sgleam/image.{type Image}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_equal")
pub fn eq(a: x, b: x) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_approx")
pub fn approx(a: Float, b: Float, tolerance: Float) -> Bool

/// Passes if `a` and `b` are the same by `image.same`, which compares the
/// pixels when the trees differ.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_same")
pub fn same(a: Image, b: Image) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_true")
pub fn true(val: Bool) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_false")
pub fn false(val: Bool) -> Bool
