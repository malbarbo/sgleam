import gleam/io
import gleam/list
import sgleam/color
import sgleam/fill
import sgleam/image
import sgleam/stroke
import sgleam/style
import sgleam/yplace

pub fn main() {
  color.all
  |> list.sized_chunk(21)
  |> list.map(stamp)
  |> list.fold(image.empty, fn(a, b) { image.beside_align(yplace.Top, a, b) })
  |> image.to_svg()
  |> io.println()
}

fn stamp(colors) {
  colors
  |> list.map(fn(name_color) {
    let #(name, color) = name_color
    let textcolor = case color == color.black {
      True -> color.white
      False -> color.black
    }
    image.text(name, 30, fill.with(textcolor))
    |> image.overlay(image.rectangle(370, 60, fill.with(color)))
  })
  |> list.fold(image.empty, image.above)
}
