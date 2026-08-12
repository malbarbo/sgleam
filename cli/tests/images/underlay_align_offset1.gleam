import gleam/io
import sgleam/fill
import sgleam/image.{circle, star_polygon, to_svg, underlay_align_offset}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  underlay_align_offset(
    star_polygon(20, 20, 3, fill.navy),
    xplace.Right,
    yplace.Bottom,
    10,
    10,
    circle(30, fill.cornflowerblue),
  )
  |> to_svg
  |> io.println
}
