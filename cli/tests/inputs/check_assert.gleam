fn double(x: Int) -> Int {
  x + x
}

fn positive(x: Int) -> Int {
  case x > 0 {
    True -> x
    False -> panic as "not positive"
  }
}

fn maybe(x: Int) -> Result(Int, Nil) {
  case x {
    0 -> Error(Nil)
    _ -> Ok(x)
  }
}

fn is_double(x: Int, y: Int) -> Nil {
  assert double(x) == y
}

pub fn double_examples() {
  assert double(2) == 4
  assert double(3) == 5
  assert positive(0) == 0
  assert double(0) == 0
}

pub fn helper_examples() {
  is_double(1, 2)
  is_double(1, 3)
}

pub fn let_assert_examples() {
  let assert Ok(x) = maybe(0)
  assert x == 0
}

pub fn panic_examples() {
  panic as "not ready"
}
