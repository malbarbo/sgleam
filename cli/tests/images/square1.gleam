import sgleam/fill
import sgleam/image.{square, to_svg}

pub fn smain() {
  square(40, fill.slateblue)
  |> to_svg
}
