# Interactive Programs

The `sgleam/world`{.gleam} module allows you to create animations and interactive programs using the "world" (big-bang) pattern: a state is transformed into an image, and user events produce a new state.

```gleam
import sgleam/world
import sgleam/image
```


## Concept

An interactive program is defined by:

1. **Initial state**: any Gleam value
2. **Drawing function**: transforms the state into an image
3. **Event functions** (optional): update the state in response to clock ticks or key presses
4. **Stop condition** (optional): defines when the program should stop


## Creating a world

```gleam
world.create(initial_state, drawing_function)
|> world.on_tick(tick_function)
|> world.on_key_press(key_function)
|> world.run()
```


## Simple animation

To create an animation without user interaction, use `world.animate`{.gleam}:

```gleam
import sgleam/fill
import sgleam/image
import sgleam/world

pub fn main() {
  world.animate(draw)
}

const max = 200

fn draw(n: Int) {
  let n = n % max
  let n = case n < max / 2 {
    True -> n
    False -> max - n
  }
  image.empty_scene(max, max)
  |> image.overlay(image.circle(n, fill.red))
}
```

The function receives the frame number (starting at 0) and returns the image to be displayed.


## Complete example: bouncing ball

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


## API reference

### Creation

```gleam
world.create(state, to_image)
```
Creates a world with an initial state and a drawing function.

```gleam
world.animate(fn(Int) -> Image)
```
Creates an animation (receives the frame number).

### Configuration

```gleam
world.tick_rate(world, rate)
```
Sets the on_tick call frequency (1 to 100 per second, default 28).

```gleam
world.on_tick(world, fn(a) -> a)
```
Sets the function called on each tick.

```gleam
world.stop_when(world, fn(a) -> Bool)
```
Sets the stop condition.

### Keyboard events

```gleam
world.on_key_press(world, fn(a, Key) -> a)
```
Key typed (character).

```gleam
world.on_key_down(world, fn(a, Key) -> a)
```
Key pressed down.

```gleam
world.on_key_up(world, fn(a, Key) -> a)
```
Key released.

### Pad events

```gleam
world.on_pad_down(world, fn(a, PadInput) -> a)
world.on_pad_up(world, fn(a, PadInput) -> a)
```

The teaching pad has `Up`, `Down`, `Left`, `Right`, `ActionA`, and `ActionB`.
Arrow keys and WASD act as its directions, Z/K as `ActionA`, and X/L as
`ActionB`. When a pad handler is set, a mapped key goes to that handler;
otherwise it remains available to the corresponding key handler.

### Execution

```gleam
world.run(world)
```
Starts the interactive program.

### Keys

Keys are represented by the `world.Key`{.gleam} type:

| Key | Value |
|-----|-------|
| Arrows | `ArrowLeft`{.gleam}, `ArrowRight`{.gleam}, `ArrowUp`{.gleam}, `ArrowDown`{.gleam} |
| Enter | `Enter`{.gleam} |
| Space | `Char(" ")`{.gleam} |
| Escape | `Escape`{.gleam} |
| Letters/numbers | `Char("a")`{.gleam}, `Char("1")`{.gleam}, etc. |
| Function keys | `F1`{.gleam} to `F12`{.gleam} |
| Others | `Backspace`{.gleam}, `Tab`{.gleam}, `Delete`{.gleam}, `Home`{.gleam}, `End`{.gleam}, `PageUp`{.gleam}, `PageDown`{.gleam} |


## Multiplayer worlds

The `sgleam/mworld` module runs one shared state for a fixed group of players.
The room chooses the players before the first frame, and they keep the same
one-based indices until the match ends. Input handlers receive the player who
sent the event, while the drawing function receives each player in turn and
may return a different image for each one.

```gleam
import sgleam/mworld

mworld.create(initial_state, min: 2, max: 4, to_image: draw)
|> mworld.on_tick(tick)
|> mworld.on_pad_down(move)
|> mworld.run()
```

The multiplayer handlers have these shapes:

```gleam
fn(a, mworld.Player, mworld.Key) -> a
fn(a, mworld.Player, mworld.PadInput) -> a
```

Use `mworld.player_index(player)` for the one-based position and
`mworld.player_nickname(player)` for the nickname selected by the room.
`mworld.tick_rate`, `mworld.on_tick`, `mworld.stop_when`, and the keyboard and
pad handlers otherwise follow the `world` API. A `world.World` is implemented
as an `mworld.MultiWorld` whose minimum and maximum player counts are both 1.
