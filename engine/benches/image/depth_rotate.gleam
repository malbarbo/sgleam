import bench
import gleam/list
import sgleam/fill
import sgleam/image

/// A row of `n` squares is a tree `n` levels deep. The last size that
/// prints shows how deep a tree can go before the stack runs out.
pub fn main() {
  let sizes = [1000, 2000, 4000, 8000, 16_000]
  bench.run_on("rotate a deep row", sizes, row, image.rotate(_, 90))
}

fn row(n: Int) -> image.Image {
  bench.range(n)
  |> list.map(fn(_) { image.square(1, fill.red) })
  |> image.combine(image.beside)
}
