fn fact(n: Int) -> Int {
  case n {
    0 -> 1
    _ -> n * fact(n - 1)
  }
}

fn loop(n: Int) -> Int {
  1 + loop(n + 1)
}

pub fn fact_examples() {
  assert fact(5) == 120
  assert fact(25) > 0
  assert fact(3) == 6
}

pub fn loop_examples() {
  assert loop(0) == 0
}
