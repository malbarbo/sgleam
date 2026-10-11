import gleam/float
import gleam/int
import gleam/list
import gleam/pair
import gleam/string
import sgleam/system

/// Runs `work` for each size in `sizes` and returns a line per size with the
/// time of one run. The factor is the time over the time of the size before,
/// so a size that doubles shows 2 for linear work and 4 for quadratic work.
pub fn run(name: String, sizes: List(Int), work: fn(Int) -> a) -> String {
  run_on(name, sizes, fn(size) { size }, work)
}

/// Like `run`, but times only `work` on what `setup` makes of the size.
pub fn run_on(
  name: String,
  sizes: List(Int),
  setup: fn(Int) -> s,
  work: fn(s) -> a,
) -> String {
  list.map_fold(sizes, 0.0, fn(before, size) {
    let input = setup(size)
    let ms = time(fn() { work(input) })
    let factor = case before >. 0.0 {
      True -> "x" <> show(ms /. before)
      False -> ""
    }
    let line =
      string.pad_end(name, 28, " ")
      <> string.pad_start(int.to_string(size), 7, " ")
      <> string.pad_start(show(ms), 11, " ")
      <> " ms  "
      <> factor
    #(ms, line)
  })
  |> pair.second
  |> string.join("\n")
}

/// The time of one run of `work` in milliseconds. A fast `work` runs again
/// until 200 ms pass, because the clock counts whole milliseconds.
fn time(work: fn() -> a) -> Float {
  let start = system.now_ms()
  repeat(work, start, 1)
}

fn repeat(work: fn() -> a, start: Int, runs: Int) -> Float {
  work()
  let elapsed = system.now_ms() - start
  case elapsed >= 200 {
    True -> int.to_float(elapsed) /. int.to_float(runs)
    False -> repeat(work, start, runs + 1)
  }
}

fn show(x: Float) -> String {
  float.to_string(int.to_float(float.round(x *. 100.0)) /. 100.0)
}

/// The numbers from 0 to `n - 1`.
pub fn range(n: Int) -> List(Int) {
  list.repeat(0, n) |> list.index_map(fn(_, i) { i })
}
