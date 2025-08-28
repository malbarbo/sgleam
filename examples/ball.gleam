import gleam/int
import sgleam/check
import sgleam/fill
import sgleam/image
import sgleam/world

const width = 600

const height = 400

const radius = 20

const min_vel = 1

const max_vel = 5

type Ball {
  Ball(x: Int, y: Int, vx: Int, vy: Int)
}

pub fn main() {
  world.create(init(), draw)
  |> world.on_tick(tick)
  |> world.on_key_down(on_key)
  |> world.run()
}

fn draw(ball: Ball) -> image.Image {
  image.empty_scene(width, height)
  |> image.place_image(ball.x, ball.y, image.circle(radius, fill.red))
}

fn init() -> Ball {
  Ball(
    radius + int.random(width - radius + 1),
    radius + int.random(height - radius + 1),
    min_vel + int.random(max_vel + 1),
    max_vel + int.random(max_vel + 1),
  )
}

fn tick(ball: Ball) -> Ball {
  let #(x, vx) = move(ball.x, ball.vx, radius, width - radius)
  let #(y, vy) = move(ball.y, ball.vy, radius, height - radius)
  Ball(x, y, vx, vy)
}

fn on_key(ball: Ball, key: world.Key) -> Ball {
  case key {
    world.ArrowRight if ball.vx > 0 -> Ball(..ball, vx: ball.vx + 1)
    world.ArrowRight -> Ball(..ball, vx: ball.vx - 1)
    world.ArrowLeft if ball.vx > 0 -> Ball(..ball, vx: ball.vx - 1)
    world.ArrowLeft -> Ball(..ball, vx: ball.vx + 1)
    world.ArrowUp if ball.vy > 0 -> Ball(..ball, vy: ball.vy + 1)
    world.ArrowUp -> Ball(..ball, vy: ball.vy - 1)
    world.ArrowDown if ball.vy > 0 -> Ball(..ball, vy: ball.vy - 1)
    world.ArrowDown -> Ball(..ball, vy: ball.vy + 1)
    world.Char("r") -> init()
    _ -> ball
  }
}

fn move(p: Int, v: Int, min: Int, max: Int) -> #(Int, Int) {
  let p = p + v
  let #(p, v) = case p < min {
    True -> #(2 * min - p, 0 - v)
    False -> #(p, v)
  }
  case p > max {
    True -> #(2 * max - p, 0 - v)
    False -> #(p, v)
  }
}

pub fn move_examples() {
  check.eq(move(10, 5, 2, 20), #(15, 5))
  check.eq(move(3, -4, 2, 20), #(5, 4))
  check.eq(move(18, 5, 2, 20), #(17, -5))
}
