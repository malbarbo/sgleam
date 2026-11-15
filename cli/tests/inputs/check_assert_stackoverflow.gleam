fn forever(n: Int) -> Int {
  1 + forever(n + 1)
}

pub fn inside_examples() {
  assert forever(0) == 1
  assert 1 + 1 == 2
}

pub fn outside_examples() {
  let n = forever(0)
  assert n == 1
}

pub fn after_examples() {
  assert 2 + 2 == 4
}
