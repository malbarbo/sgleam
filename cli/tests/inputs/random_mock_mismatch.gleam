import sgleam/random

pub fn main() {
  let gen = random.mock([random.int_returns(1, 6, 4)])
  random.pick(gen, ["a", "b"])
}
