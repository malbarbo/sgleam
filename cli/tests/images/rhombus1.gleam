import sgleam/fill
import sgleam/image.{rhombus, to_svg}

pub fn smain() {
  rhombus(40, 45, fill.magenta)
  |> to_svg
}
