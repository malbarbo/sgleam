import sgleam/env.{type Env}
import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn init(env: Env) -> Int {
  echo env.args(env)
  1
}

pub fn smain() {
  world.create_from(init, fn(state) { image.circle(state, stroke.red) })
  |> world.on_tick(fn(state) { state + 1 })
  |> world.stop_when(fn(state) { state == 3 })
}
