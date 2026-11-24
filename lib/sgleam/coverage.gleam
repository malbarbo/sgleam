//// The marks that a run with coverage puts at the start of each region of
//// the program. The engine writes the calls, and the user never imports this
//// module.

@external(javascript, "../sgleam/sgleam_ffi.mjs", "coverage_hit")
pub fn hit(n: Float) -> Nil

/// Returns `True`, after it marks the region `n` as run. The engine puts
/// it before the right operand of `&&`.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "coverage_hit_true")
pub fn hit_true(n: Float) -> Bool

/// Returns `False`, after it marks the region `n` as run. The engine puts
/// it before the right operand of `||`.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "coverage_hit_false")
pub fn hit_false(n: Float) -> Bool
