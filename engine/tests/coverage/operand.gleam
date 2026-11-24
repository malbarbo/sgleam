import sgleam/check

pub fn positive(n: Int) -> Bool {
  n > 0 && panic as "positive"
}

pub fn positive_examples() {
  check.eq(positive(1), True)
}
