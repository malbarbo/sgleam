import gleam/list

pub fn main() {
  f(1)
}

fn f(x: Int) -> Int {
  list.fold([x], 0, fn(acc, y) { acc + f(y) })
}
