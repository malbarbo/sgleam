import gleam/int
import gleam/list
import gleam/string

pub fn smain(lines: List(String)) {
  let nums = int.range(1, list.length(lines) + 1, [], list.prepend) |> list.reverse
  list.zip(nums, lines)
  |> list.map(fn(p) { int.to_string(p.0) <> " " <> p.1 })
  |> string.join("\n")
}
