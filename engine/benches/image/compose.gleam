import bench
import gleam/list
import sgleam/fill
import sgleam/image

pub fn main() {
  let sizes = [125, 250, 500]
  bench.run("beside equal squares", sizes, fn(n) {
    bench.range(n)
    |> list.map(fn(_) { image.square(10, fill.red) })
    |> image.combine(image.beside)
  })
  bench.run("beside growing squares", sizes, fn(n) {
    bench.range(n)
    |> list.map(fn(i) { image.square(i, fill.red) })
    |> image.combine(image.beside)
  })
  bench.run("above growing squares", sizes, fn(n) {
    bench.range(n)
    |> list.map(fn(i) { image.square(i, fill.red) })
    |> image.combine(image.above)
  })
  bench.run("overlay_offset acc below", sizes, fn(n) {
    bench.range(n)
    |> list.fold(image.empty, fn(acc, _) {
      image.overlay_offset(image.square(10, fill.red), 1, 1, acc)
    })
  })
  bench.run("overlay_xy negative", sizes, fn(n) {
    bench.range(n)
    |> list.fold(image.empty, fn(acc, _) {
      image.overlay_xy(acc, -1, -1, image.square(10, fill.red))
    })
  })
  bench.run("grid of n by n circles", [10, 20, 40], fn(n) {
    let row =
      bench.range(n)
      |> list.map(fn(_) { image.circle(5, fill.blue) })
      |> image.combine(image.beside)
    bench.range(n)
    |> list.map(fn(_) { row })
    |> image.combine(image.above)
  })
}
