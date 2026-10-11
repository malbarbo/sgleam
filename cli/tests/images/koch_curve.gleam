import sgleam/fill
import sgleam/image.{beside_align, rotate, square, to_svg}
import sgleam/yplace

pub fn smain() {
  koch_curve(5)
  |> to_svg
}

fn koch_curve(n) {
  case n <= 0 {
    True -> square(1, fill.black)
    False -> {
      let smaller = koch_curve(n - 1)
      smaller
      |> beside_align(yplace.Bottom, rotate(smaller, 60))
      |> beside_align(yplace.Bottom, rotate(smaller, -60))
      |> beside_align(yplace.Bottom, smaller)
    }
  }
}
