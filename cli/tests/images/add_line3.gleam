import gleam/io
import sgleam/fill
import sgleam/image.{add_line, rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn main() {
  rectangle(100, 100, fill.darkolivegreen)
  |> add_line(
    25,
    25,
    75,
    75,
    style.join([
      stroke.goldenrod,
      stroke.width(30),
      stroke.linejoin_round,
      stroke.linecap_round,
    ]),
  )
  |> to_svg
  |> io.println
}
