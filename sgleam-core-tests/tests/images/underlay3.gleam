import gleam/io
import sgleam/fill
import sgleam/image.{combine, ellipse, to_svg, underlay}
import sgleam/style

pub fn main() {
  [
    ellipse(10, 60, style.join([fill.red, fill.opacity(0.2)])),
    ellipse(20, 50, style.join([fill.red, fill.opacity(0.2)])),
    ellipse(30, 40, style.join([fill.red, fill.opacity(0.2)])),
    ellipse(40, 30, style.join([fill.red, fill.opacity(0.2)])),
    ellipse(50, 20, style.join([fill.red, fill.opacity(0.2)])),
    ellipse(60, 10, style.join([fill.red, fill.opacity(0.2)])),
  ]
  |> combine(underlay)
  |> to_svg
  |> io.println
}
