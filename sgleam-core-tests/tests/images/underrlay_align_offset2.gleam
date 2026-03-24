import gleam/io
import sgleam/fill
import sgleam/image.{rhombus, star_polygon, to_svg, underlay_align_offset}
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  let star = star_polygon(20, 11, 3, fill.cornflowerblue)
  rhombus(120, 90, fill.navy)
  |> underlay_align_offset(xplace.Left, yplace.Top, 16, 16, star)
  |> underlay_align_offset(xplace.Right, yplace.Top, -16, 16, star)
  |> underlay_align_offset(xplace.Left, yplace.Bottom, 16, -16, star)
  |> underlay_align_offset(xplace.Right, yplace.Bottom, -16, -16, star)
  |> to_svg
  |> io.println
}
