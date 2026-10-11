import sgleam/fill
import sgleam/image.{rhombus, to_svg}

pub fn smain() {
  rhombus(80, 150, fill.mediumpurple)
  |> to_svg
}
