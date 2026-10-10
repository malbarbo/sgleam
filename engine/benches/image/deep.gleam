import bench
import gleam/list
import sgleam/fill
import sgleam/image

/// A row of `n` squares is a tree `n` levels deep. The largest size shows
/// how deep a tree can go before the stack runs out.
pub fn main() {
  let sizes = [5000, 10_000, 20_000]
  bench.run("build a deep row", sizes, row)
  bench.run_on("to_svg of a deep row", sizes, row, image.to_svg)
  bench.run_on("rotate a deep row", sizes, row, image.rotate(_, 90))
}

fn row(n: Int) -> image.Image {
  bench.range(n)
  |> list.map(fn(_) { image.square(1, fill.red) })
  |> image.combine(image.beside)
}
