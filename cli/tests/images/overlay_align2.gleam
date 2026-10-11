import sgleam/fill
import sgleam/image.{overlay_align, square, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn smain() {
  square(20, fill.silver)
  |> overlay_align(xplace.Right, yplace.Bottom, square(30, fill.seagreen))
  |> overlay_align(xplace.Right, yplace.Bottom, square(40, fill.silver))
  |> overlay_align(xplace.Right, yplace.Bottom, square(50, fill.seagreen))
  |> to_svg
}
