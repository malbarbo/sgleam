import sgleam/image.{rectangle, to_svg}
import sgleam/stroke
import sgleam/style

pub fn smain() {
  rectangle(6, 4, style.join([stroke.black, stroke.width(1)]))
  |> to_svg
}
