import sgleam/fill
import sgleam/image.{rectangle, to_svg}

pub fn smain() {
  rectangle(20, 40, fill.black)
  |> to_svg
}
