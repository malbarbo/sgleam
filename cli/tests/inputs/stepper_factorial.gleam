pub fn fat(n) {
  case n == 0 {
    True -> 1
    False -> n * fat(n - 1)
  }
}
