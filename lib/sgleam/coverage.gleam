//// The marks that a run with coverage puts at the start of each region of
//// the program. The engine writes the calls, and the user never imports this
//// module.

@external(javascript, "../sgleam/sgleam_ffi.mjs", "coverage_hit")
pub fn hit(n: Float) -> Nil

/// Returns `b`, after it marks the region `n` as run.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "coverage_hit_bool")
pub fn hit_bool(n: Float, b: Bool) -> Bool
