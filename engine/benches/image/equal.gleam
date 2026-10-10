import bench
import gleam/list
import sgleam/fill
import sgleam/image

pub fn main() {
  bench.run_on(
    "== of equal rows",
    [1000, 2000, 4000],
    fn(n) { #(row(n), row(n)) },
    fn(pair) { pair.0 == pair.1 },
  )
}

fn row(n: Int) -> image.Image {
  bench.range(n)
  |> list.map(fn(_) { image.square(10, fill.red) })
  |> image.combine(image.beside)
}
