import gleam/io
import sgleam/color
import sgleam/fill
import sgleam/image.{above, beside, combine, square, to_svg}

pub fn main() {
  [
    color.rgb(51, 0, 255),
    color.rgb(102, 0, 255),
    color.rgb(153, 0, 255),
    color.rgb(204, 0, 255),
    color.rgb(255, 0, 255),
    color.rgb(255, 204, 0),
  ]
  |> colored_carpet
  |> to_svg
  |> io.println
}

fn colored_carpet(colors) {
  case colors {
    [] -> image.empty
    [color] -> square(1, fill.with(color))
    [first, ..rest] -> {
      let c = colored_carpet(rest)
      let i = square(image.width(c), fill.with(first))
      combine(
        [
          combine([c, c, c], beside),
          combine([c, i, c], beside),
          combine([c, c, c], beside),
        ],
        above,
      )
    }
  }
}
