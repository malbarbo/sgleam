import gleam/list

pub fn main() {
  list.map([1, 2, 3], floop)
}

fn floop(x) {
  x + floop(x)
}
