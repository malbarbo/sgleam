import sgleam/check

fn f(x: Int) -> Int {
  case x {
    0 -> todo
    1 -> todo as "not implemented"
    2 -> panic
    3 -> panic as "invalid input"
    4 -> 1 + f(4)
    _ -> x
  }
}

pub fn f_examples() {
  check.eq(f(0), 0)
  check.eq(f(1), 1)
  check.eq(f(2), 3)
  check.eq(f(3), 3)
  check.eq(f(4), 4)
  check.eq(f(5), 5)
}
