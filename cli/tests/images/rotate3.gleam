import sgleam/fill
import sgleam/image.{beside, rectangle, rotate, to_svg}

pub fn smain() {
  rectangle(40, 20, fill.darkseagreen)
  |> beside(rectangle(20, 100, fill.darkseagreen))
  |> rotate(45)
  |> to_svg
}
