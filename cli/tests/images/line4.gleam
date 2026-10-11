import sgleam/image.{beside, line, to_svg}
import sgleam/stroke

pub fn smain() {
  beside(line(0, 20, stroke.black), line(20, 0, stroke.red))
  |> to_svg
}
