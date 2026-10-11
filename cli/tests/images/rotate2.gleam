import sgleam/image.{rectangle, rotate, to_svg}
import sgleam/stroke

pub fn smain() {
  rectangle(50, 50, stroke.black)
  |> rotate(5)
  |> to_svg
}
