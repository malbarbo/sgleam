import gleam/float
import gleam/int
import gleam/list

/// A generator of pseudo-random numbers. A generator is a value, so the same
/// generator always gives the same number. Each function returns the next
/// generator together with the number, and the next draw has to use it.
pub opaque type Generator {
  Generator(state: Int)
}

/// Creates a generator from `seed`. The same seed gives the same sequence.
pub fn new(seed: Int) -> Generator {
  Generator(seed)
}

/// Draws an `Int` between `from` and `to`, both included.
pub fn int(gen: Generator, from: Int, to: Int) -> #(Int, Generator) {
  let low = int.min(from, to)
  let size = int.absolute_value(to - from) + 1
  let #(x, gen) = float(gen)
  // The product rounds up to `size` when `size` is close to 2^53.
  let offset = int.min(float.truncate(x *. int.to_float(size)), size - 1)
  #(low + offset, gen)
}

/// Draws a `Float` from 0.0, included, to 1.0, excluded.
pub fn float(gen: Generator) -> #(Float, Generator) {
  let #(x, state) = step(gen.state)
  #(x, Generator(state))
}

/// Draws an item of `items`, or `Error(Nil)` when `items` is empty.
pub fn pick(gen: Generator, items: List(a)) -> #(Result(a, Nil), Generator) {
  case items {
    [] -> #(Error(Nil), gen)
    _ -> {
      let #(index, gen) = int(gen, 0, list.length(items) - 1)
      #(items |> list.drop(index) |> list.first, gen)
    }
  }
}

/// Puts `items` in a random order.
pub fn shuffle(gen: Generator, items: List(a)) -> #(List(a), Generator) {
  let #(gen, keyed) =
    list.map_fold(items, gen, fn(gen, item) {
      let #(key, gen) = float(gen)
      #(gen, #(key, item))
    })
  let items =
    keyed
    |> list.sort(fn(a, b) { float.compare(a.0, b.0) })
    |> list.map(fn(pair) { pair.1 })
  #(items, gen)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "random_step")
fn step(state: Int) -> #(Float, Int)
