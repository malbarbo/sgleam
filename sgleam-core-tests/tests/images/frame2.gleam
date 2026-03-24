import gleam/io
import sgleam/fill
import sgleam/image.{beside, ellipse, frame, to_svg}

pub fn main() {
  ellipse(20, 70, fill.lightsteelblue)
  |> beside(ellipse(20, 50, fill.mediumslateblue) |> frame)
  |> beside(ellipse(20, 30, fill.slateblue))
  |> beside(ellipse(20, 10, fill.navy))
  |> to_svg
  |> io.println
}
