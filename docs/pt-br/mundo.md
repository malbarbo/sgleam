# Programas Interativos

O módulo `sgleam/world`{.gleam} permite criar animações e programas interativos usando o padrão "mundo" (big-bang): um estado é transformado em uma imagem, e eventos do usuário produzem um novo estado.

```gleam
import sgleam/world
import sgleam/image
```


## Conceito

Um programa interativo é definido por:

1. **Estado inicial**: qualquer valor Gleam
2. **Função de desenho**: transforma o estado em uma imagem
3. **Funções de evento** (opcionais): atualizam o estado em resposta a ticks do relógio ou teclas pressionadas
4. **Condição de parada** (opcional): define quando o programa deve parar


## Criando um mundo

```gleam
world.create(estado_inicial, funcao_de_desenho)
|> world.on_tick(funcao_de_tick)
|> world.on_key_press(funcao_de_tecla)
|> world.run()
```


## Animação simples

Para criar uma animação sem interação do usuário, use `world.animate`{.gleam}:

```gleam
import sgleam/fill
import sgleam/image
import sgleam/world

pub fn main() {
  world.animate(desenho)
}

const max = 200

fn desenho(n: Int) {
  let n = n % max
  let n = case n < max / 2 {
    True -> n
    False -> max - n
  }
  image.empty_scene(max, max)
  |> image.overlay(image.circle(n, fill.red))
}
```

A função recebe o número do quadro (iniciando em 0) e retorna a imagem a ser exibida.


## Exemplo completo: bola quicando

```gleam
import gleam/int
import sgleam/fill
import sgleam/image
import sgleam/world

const width = 600
const height = 400
const radius = 20

type Ball {
  Ball(x: Int, y: Int, vx: Int, vy: Int)
}

pub fn main() {
  world.create(Ball(100, 100, 3, 4), draw)
  |> world.on_tick(tick)
  |> world.on_key_down(on_key)
  |> world.run()
}

fn draw(ball: Ball) -> image.Image {
  image.empty_scene(width, height)
  |> image.place_image(ball.x, ball.y, image.circle(radius, fill.red))
}

fn tick(ball: Ball) -> Ball {
  let x = ball.x + ball.vx
  let y = ball.y + ball.vy
  let vx = case x < radius || x > width - radius {
    True -> 0 - ball.vx
    False -> ball.vx
  }
  let vy = case y < radius || y > height - radius {
    True -> 0 - ball.vy
    False -> ball.vy
  }
  Ball(x, y, vx, vy)
}

fn on_key(ball: Ball, key: world.Key) -> Ball {
  case key {
    world.Char("r") -> Ball(100, 100, 3, 4)
    _ -> ball
  }
}
```


## API de referência

### Criação

```gleam
world.create(state, to_image)
```
Cria um mundo com estado inicial e função de desenho.

```gleam
world.animate(fn(Int) -> Image)
```
Cria uma animação (recebe o número do quadro).

### Configuração

```gleam
world.tick_rate(world, rate)
```
Define a frequência de chamada de on_tick (1 a 100 por segundo, padrão 28).

```gleam
world.on_tick(world, fn(a) -> a)
```
Define a função chamada a cada tick.

```gleam
world.stop_when(world, fn(a) -> Bool)
```
Define a condição de parada.

### Eventos de teclado

```gleam
world.on_key_press(world, fn(a, Key) -> a)
```
Tecla digitada (caractere).

```gleam
world.on_key_down(world, fn(a, Key) -> a)
```
Tecla pressionada.

```gleam
world.on_key_up(world, fn(a, Key) -> a)
```
Tecla solta.

### Eventos de controle

```gleam
world.on_pad_down(world, fn(a, PadInput) -> a)
world.on_pad_up(world, fn(a, PadInput) -> a)
```

O controle didático possui `Up`, `Down`, `Left`, `Right`, `ActionA` e
`ActionB`. As setas e WASD funcionam como direções, Z/K como `ActionA` e X/L
como `ActionB`. Quando existe um handler de controle, uma tecla mapeada é
enviada para ele; caso contrário, continua disponível para o handler de
teclado correspondente.

### Execução

```gleam
world.run(world)
```
Inicia o programa interativo.

### Teclas

As teclas são representadas pelo tipo `world.Key`{.gleam}:

| Tecla | Valor |
|-------|-------|
| Setas | `ArrowLeft`{.gleam}, `ArrowRight`{.gleam}, `ArrowUp`{.gleam}, `ArrowDown`{.gleam} |
| Enter | `Enter`{.gleam} |
| Espaço | `Char(" ")`{.gleam} |
| Escape | `Escape`{.gleam} |
| Letras/números | `Char("a")`{.gleam}, `Char("1")`{.gleam}, etc. |
| Funções | `F1`{.gleam} a `F12`{.gleam} |
| Outros | `Backspace`{.gleam}, `Tab`{.gleam}, `Delete`{.gleam}, `Home`{.gleam}, `End`{.gleam}, `PageUp`{.gleam}, `PageDown`{.gleam} |


## Mundos multijogador

O módulo `sgleam/mworld` executa um único estado compartilhado para um grupo
fixo de jogadores. A sala escolhe os jogadores antes do primeiro quadro, e
eles mantêm os mesmos índices, iniciados em 1, até o fim da partida. Os
handlers de entrada recebem o jogador que enviou o evento, enquanto a função
de desenho recebe cada jogador por vez e pode produzir uma imagem diferente
para cada um.

```gleam
import sgleam/mworld

mworld.create(estado_inicial, min: 2, max: 4, to_image: desenhar)
|> mworld.on_tick(tick)
|> mworld.on_pad_down(mover)
|> mworld.run()
```

Os handlers multijogador têm as seguintes formas:

```gleam
fn(a, mworld.Player, mworld.Key) -> a
fn(a, mworld.Player, mworld.PadInput) -> a
```

Use `mworld.player_index(player)` para obter a posição iniciada em 1 e
`mworld.player_nickname(player)` para obter o apelido escolhido pela sala.
`mworld.tick_rate`, `mworld.on_tick`, `mworld.stop_when` e os handlers de
teclado e controle seguem o restante da API de `world`. Um `world.World` é
implementado como um `mworld.MultiWorld` cujo mínimo e máximo de jogadores são
ambos 1.
