import sgleam/check

pub fn approx_examples() {
  check.approx(1.2, 1.1, 0.1)
  check.approx(1.2, 1.1, 0.01)
  check.approx(1.2, 1.1, 0.2)
}
