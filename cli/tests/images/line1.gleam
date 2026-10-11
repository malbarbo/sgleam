import sgleam/image.{line, to_svg}
import sgleam/stroke

pub fn smain() {
  line(30, 30, stroke.black)
  |> to_svg
}
