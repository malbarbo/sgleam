pub fn main() {
  echo fact(25)
}

fn fact(n: Int) -> Int {
  case n {
    0 -> 1
    _ -> n * fact(n - 1)
  }
}
