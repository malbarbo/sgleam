import sgleam/fill
import sgleam/image.{text, to_svg}

pub fn smain() {
  text("Testing text", 16, fill.black)
  |> to_svg
}
