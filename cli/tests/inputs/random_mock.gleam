import sgleam/random

pub fn smain() {
  let gen =
    random.mock([
      random.int_returns(1, 6, 4),
      random.int_returns(1, 6, 2),
      random.float_returns(0.5),
      random.pick_returns(["a", "b", "c"], Ok("c")),
      random.pick_returns([], Error(Nil)),
      random.shuffle_returns([1, 1, 2], [1, 2, 1]),
    ])
  let #(a, gen) = random.int(gen, 1, 6)
  let #(b, gen) = random.int(gen, 1, 6)
  let #(c, gen) = random.float(gen)
  let #(d, gen) = random.pick(gen, ["a", "b", "c"])
  let #(e, gen): #(Result(Int, Nil), _) = random.pick(gen, [])
  let #(f, _) = random.shuffle(gen, [1, 1, 2])
  #(a, b, c, d, e, f)
}
