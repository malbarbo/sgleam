import sgleam/image.{square, to_svg}
import sgleam/stroke

pub fn smain() {
  square(50, stroke.darkmagenta)
  |> to_svg
}
