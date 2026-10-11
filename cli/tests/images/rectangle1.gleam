import sgleam/image.{rectangle, to_svg}
import sgleam/stroke

pub fn smain() {
  rectangle(40, 20, stroke.black)
  |> to_svg
}
