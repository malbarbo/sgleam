pub const one = 1

pub fn two() {
  2
}

pub type Three {
  Num3
}

pub type Pair {
  Pair(Int, Int)
}

/// Shares the module's own short name.
pub fn user() {
  "self"
}

/// Shares the name of a stdlib module.
pub fn list(x: Int) -> Int {
  x
}

/// Raises an error outside the repl's own module.
pub fn boom() {
  panic as "boom"
}
