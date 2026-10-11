import gleam/list
import gleam/string
import sgleam/image.{flip_horizontal, line, rotate, scale, scalef, to_svg}
import sgleam/stroke

pub fn smain() {
  let l = line(6, 0, stroke.black)
  [
    rotate(l, 45),
    rotate(l, -45),
    rotate(l, 30),
    rotate(line(0, 0, stroke.black), 45),
    scale(l, 4),
    scalef(l, 0.5),
    flip_horizontal(scale(line(6, 4, stroke.black), 2)),
  ]
  |> list.map(to_svg)
  |> string.join("\n")
}
