import sgleam/check

fn double(x: Int) -> Int {
  x + x
}

fn maybe(x: Int) -> Result(Int, Nil) {
  case x {
    0 -> Error(Nil)
    _ -> Ok(x)
  }
}

pub fn assert_examples() {
  assert double(2) == 5
}

pub fn panic_examples() {
  panic as "not ready"
}

pub fn let_assert_examples() {
  let assert Ok(x) = maybe(0)
  check.eq(x, 0)
}

pub fn double_examples() {
  check.eq(double(2), 4)
}
