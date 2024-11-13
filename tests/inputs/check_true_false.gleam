import sgleam/check

pub fn eq_examples() {
  check.true(2 == 1 + 1)
  check.true(2 == 3)
  check.false(2 == 1 + 1)
  check.false(2 == 3)
}
