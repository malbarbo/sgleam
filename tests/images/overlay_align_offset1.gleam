import gleam/io
import sgleam/fill
import sgleam/image.{circle, overlay_align_offset, star_polygon, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  overlay_align_offset(
    xplace.Right,
    yplace.Bottom,
    star_polygon(20, 20, 3, fill.navy),
    10,
    10,
    circle(30, fill.cornflowerblue),
  )
  |> to_svg
  |> io.println
}
