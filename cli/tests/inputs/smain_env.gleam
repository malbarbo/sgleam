import sgleam/env.{type Env}
import sgleam/random

pub fn smain(env: Env) {
  let #(a, gen) = random.int(random.new(42), 1, 100)
  let #(b, gen) = random.float(gen)
  let #(c, gen) = random.pick(gen, ["a", "b", "c"])
  let #(d, _) = random.shuffle(gen, [1, 2, 3, 4, 5])
  #(env.args(env), env.time(env) > 0, a, b, c, d)
}
