import gleam/io
import sgleam/fill
import sgleam/image.{square, to_svg, underlay_align}
import sgleam/style
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  square(50, style.join([fill.seagreen, fill.opacityf(0.25)]))
  |> underlay_align(
    xplace.Left, yplace.Middle, square(40, style.join([fill.seagreen, fill.opacityf(0.25)])),
  )
  |> underlay_align(
    xplace.Left, yplace.Middle, square(30, style.join([fill.seagreen, fill.opacityf(0.25)])),
  )
  |> underlay_align(
    xplace.Left, yplace.Middle, square(20, style.join([fill.seagreen, fill.opacityf(0.25)])),
  )
  |> to_svg
  |> io.println
}
