import sgleam/fill
import sgleam/image.{beside, crop_align, frame, square, to_svg}
import sgleam/xplace
import sgleam/yplace

pub fn smain() {
  let square = square(10, fill.red)
  beside(
    beside(
      frame(crop_align(square, xplace.Left, yplace.Top, 20, 20)),
      frame(crop_align(square, xplace.Center, yplace.Middle, 20, 20)),
    ),
    frame(crop_align(square, xplace.Right, yplace.Bottom, 20, 20)),
  )
  |> to_svg
}
