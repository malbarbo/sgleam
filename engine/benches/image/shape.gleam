import bench
import gleam/list
import sgleam/fill
import sgleam/image

pub fn main() {
  bench.run("regular_polygon by sides", [50, 100, 200], fn(n) {
    image.regular_polygon(30, n, fill.red)
  })
  bench.run("polygon by points", [50, 100, 200], fn(n) {
    bench.range(n)
    |> list.map(fn(i) { image.Point(i, { i * 37 } % 100) })
    |> image.polygon(fill.red)
  })
  bench.run("star", [1], fn(_) { image.star(30, fill.red) })
  bench.run("triangle", [1], fn(_) { image.triangle(30, fill.red) })
}
