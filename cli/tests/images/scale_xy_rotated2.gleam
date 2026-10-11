import sgleam/fill
import sgleam/image.{
  beside, crop, frame, rectangle, rotate, scale_xy, text, to_svg,
}

pub fn smain() {
  let t = rotate(text("abc", 20, fill.black), 45)
  let c = rotate(crop(rectangle(40, 40, fill.blue), 0, 0, 30, 10), 45)
  beside(
    beside(frame(t), frame(scale_xy(t, 3, 1))),
    beside(frame(c), frame(scale_xy(c, 3, 1))),
  )
  |> to_svg
}
