import bench
import gleam/list
import gleam/string
import sgleam/fill
import sgleam/image

/// A row of `n` squares is a tree `n` levels deep. The last size that
/// prints shows how deep a tree can go before the stack runs out.
pub fn smain() {
  let sizes = [1000, 2000, 4000, 8000, 16_000]
  [
    bench.run("build a deep row", sizes, row),
    bench.run_on("to_svg of a deep row", sizes, row, image.to_svg),
  ]
  |> string.join("\n")
}

fn row(n: Int) -> image.Image {
  bench.range(n)
  |> list.map(fn(_) { image.square(1, fill.red) })
  |> image.combine(image.beside)
}
