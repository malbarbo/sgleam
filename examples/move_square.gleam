import gleam/int
import sgleam/fill
import sgleam/image
import sgleam/stroke
import sgleam/style
import sgleam/world
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  world.create(Posicao(linhas / 2, colunas / 2), desenho)
  |> world.on_key(move)
  |> world.run()
}

const linhas = 8

const colunas = 10

const size = 30

pub type Posicao {
  Posicao(linha: Int, coluna: Int)
}

pub fn desenho(p: Posicao) -> image.Image {
  image.empty_scene(size * colunas, size * linhas)
  |> image.place_image_align(
    size * p.coluna,
    size * p.linha,
    xplace.Left,
    yplace.Top,
    image.square(size, [fill.red, stroke.black] |> style.join),
  )
}

pub fn move(p: Posicao, key: world.Key) -> Posicao {
  let p = case key {
    world.Left -> Posicao(..p, coluna: p.coluna - 1)
    world.Right -> Posicao(..p, coluna: p.coluna + 1)
    world.Down -> Posicao(..p, linha: p.linha - 1)
    world.Up -> Posicao(..p, linha: p.linha + 1)
    _ -> p
  }
  Posicao(int.clamp(p.linha, 0, linhas), int.clamp(p.coluna, 0, colunas))
}
