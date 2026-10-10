import bench
import gleam/list
import sgleam/fill
import sgleam/image

pub fn main() {
  let sizes = [125, 250, 500]
  bench.run_on("rotate a row", sizes, row, image.rotate(_, 30))
  bench.run_on("scale a row", sizes, row, image.scale(_, 2))
  bench.run_on("flip_horizontal a row", sizes, row, image.flip_horizontal)
  bench.run("rotate in a fold", [125, 250, 500], fn(n) {
    bench.range(n)
    |> list.fold(image.empty, fn(acc, _) {
      image.rotate(image.beside(acc, image.square(10, fill.red)), 1)
    })
  })
}

fn row(n: Int) -> image.Image {
  bench.range(n)
  |> list.map(fn(_) { image.square(10, fill.red) })
  |> image.combine(image.beside)
}
