/// The prelude is a module of its own, which no file of the user's provides.
import gleam

pub fn main() {
  let n: gleam.Int = 21
  echo n * 2
}
