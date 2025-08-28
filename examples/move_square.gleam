import gleam/int
import sgleam/fill
import sgleam/image
import sgleam/stroke
import sgleam/style
import sgleam/world
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  world.create(Pos(lines / 2, columns / 2), draw)
  |> world.on_key_down(move)
  |> world.stop_when(fn(p) { p.line == 0 && p.column == 0 })
  |> world.run()
}

const lines = 9

const columns = 11

const size = 30

pub type Pos {
  Pos(line: Int, column: Int)
}

pub fn draw(p: Pos) -> image.Image {
  image.empty_scene(size * columns, size * lines)
  |> image.place_image_align(
    size * p.column,
    size * p.line,
    xplace.Left,
    yplace.Top,
    image.square(size, [fill.red, stroke.black] |> style.join),
  )
}

pub fn move(p: Pos, key: world.Key) -> Pos {
  let p = case key {
    world.ArrowLeft -> Pos(..p, column: p.column - 1)
    world.ArrowRight -> Pos(..p, column: p.column + 1)
    world.ArrowDown -> Pos(..p, line: p.line + 1)
    world.ArrowUp -> Pos(..p, line: p.line - 1)
    _ -> p
  }
  Pos(int.clamp(p.line, 0, lines - 1), int.clamp(p.column, 0, columns - 1))
}
