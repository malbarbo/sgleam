import gleam/float
import gleam/string

pub fn main() {
  let infinity = 1.0e300 *. 1.0e300
  echo infinity
  echo string.inspect(infinity)
  echo float.to_string(0.0 -. infinity)
  echo string.inspect(infinity /. infinity)
}
