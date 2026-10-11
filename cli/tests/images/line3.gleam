import sgleam/image.{line, to_svg}
import sgleam/stroke

pub fn smain() {
  line(30, -20, stroke.red)
  |> to_svg
}
