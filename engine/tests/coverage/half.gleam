import sgleam/check

pub fn half(n: Int) -> Int {
  case n % 2 == 0 {
    True -> n / 2
    False -> panic as "odd"
  }
}

pub fn half_examples() {
  check.eq(half(4), 3)
}

pub fn odd_examples() {
  check.eq(half(3), 1)
}
