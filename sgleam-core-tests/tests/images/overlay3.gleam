import gleam/io
import sgleam/fill
import sgleam/image.{overlay, regular_polygon, to_svg}

pub fn main() {
  regular_polygon(20, 5, fill.rgb(50, 50, 255))
  |> overlay(regular_polygon(26, 5, fill.rgb(100, 100, 255)))
  |> overlay(regular_polygon(32, 5, fill.rgb(150, 150, 255)))
  |> overlay(regular_polygon(38, 5, fill.rgb(200, 200, 255)))
  |> overlay(regular_polygon(44, 5, fill.rgb(250, 250, 255)))
  |> to_svg
  |> io.println
}
