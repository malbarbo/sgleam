import gleam/io
import sgleam/check

// The name of a test is printed before the test runs.
pub fn first_examples() {
  io.println("during the first test")
  check.eq(1 + 1, 2)
}

pub fn second_examples() {
  check.eq(2 + 2, 5)
}
