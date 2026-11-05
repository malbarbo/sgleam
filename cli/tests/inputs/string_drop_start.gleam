import gleam/string

pub fn main() {
  echo string.drop_start("ação", 2)
  echo string.drop_start("😀abc", 1)
}
