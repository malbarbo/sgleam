import gleam/list

pub fn main() {
  f([1])
}

fn f(xs: List(Int)) -> Int {
  list.fold(xs, 0, fn(acc, _) { acc + f(xs) })
}
