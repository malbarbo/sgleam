import gleam/dynamic.{type Dynamic}
import gleam/float
import gleam/int
import gleam/list
import gleam/string

/// A generator of pseudo-random numbers. A generator is a value, so the same
/// generator always gives the same number. Each function returns the next
/// generator together with the number, and the next draw has to use it.
pub opaque type Generator {
  Seeded(state: Int)
  Mock(calls: List(Call))
}

/// A call that a mock generator expects, with the result it returns.
pub opaque type Call {
  IntCall(from: Int, to: Int, result: Int)
  FloatCall(result: Float)
  PickCall(items: Dynamic, result: Dynamic)
  ShuffleCall(items: Dynamic, result: Dynamic)
}

/// Creates a generator from `seed`. The same seed gives the same sequence.
pub fn new(seed: Int) -> Generator {
  Seeded(seed)
}

/// Creates a generator for tests that answers `calls` in order. Each draw has
/// to match the first call of the list, or the program panics.
pub fn mock(calls: List(Call)) -> Generator {
  Mock(calls)
}

/// The call `int(gen, from, to)`, which returns `result`.
pub fn int_returns(from: Int, to: Int, result: Int) -> Call {
  case int.min(from, to) <= result && result <= int.max(from, to) {
    True -> IntCall(from, to, result)
    False ->
      panic as {
        "random.int_returns: "
        <> int.to_string(result)
        <> " is not between "
        <> int.to_string(from)
        <> " and "
        <> int.to_string(to)
      }
  }
}

/// The call `float(gen)`, which returns `result`.
pub fn float_returns(result: Float) -> Call {
  case result >=. 0.0 && result <. 1.0 {
    True -> FloatCall(result)
    False ->
      panic as {
        "random.float_returns: "
        <> float.to_string(result)
        <> " is not in [0.0, 1.0)"
      }
  }
}

/// The call `pick(gen, items)`, which returns `result`.
pub fn pick_returns(items: List(a), result: Result(a, Nil)) -> Call {
  let valid = case result {
    Error(Nil) -> items == []
    Ok(item) -> list.contains(items, item)
  }
  case valid {
    True -> PickCall(coerce(items), coerce(result))
    False ->
      panic as {
        "random.pick_returns: "
        <> string.inspect(result)
        <> " is not a pick of "
        <> string.inspect(items)
      }
  }
}

/// The call `shuffle(gen, items)`, which returns `result`.
pub fn shuffle_returns(items: List(a), result: List(a)) -> Call {
  case is_permutation(items, result) {
    True -> ShuffleCall(coerce(items), coerce(result))
    False ->
      panic as {
        "random.shuffle_returns: "
        <> string.inspect(result)
        <> " is not a permutation of "
        <> string.inspect(items)
      }
  }
}

/// Draws an `Int` between `from` and `to`, both included.
pub fn int(gen: Generator, from: Int, to: Int) -> #(Int, Generator) {
  case gen {
    Seeded(_) -> {
      let low = int.min(from, to)
      let size = int.absolute_value(to - from) + 1
      let #(x, gen) = float(gen)
      // The product rounds up to `size` when `size` is close to 2^53.
      let offset = int.min(float.truncate(x *. int.to_float(size)), size - 1)
      #(low + offset, gen)
    }
    Mock([IntCall(f, t, result), ..calls]) if f == from && t == to -> #(
      result,
      Mock(calls),
    )
    Mock(calls) -> mismatch(calls, int_text(from, to))
  }
}

/// Draws a `Float` from 0.0, included, to 1.0, excluded.
pub fn float(gen: Generator) -> #(Float, Generator) {
  case gen {
    Seeded(state) -> {
      let #(x, state) = step(state)
      #(x, Seeded(state))
    }
    Mock([FloatCall(result), ..calls]) -> #(result, Mock(calls))
    Mock(calls) -> mismatch(calls, "random.float()")
  }
}

/// Draws an item of `items`, or `Error(Nil)` when `items` is empty.
pub fn pick(gen: Generator, items: List(a)) -> #(Result(a, Nil), Generator) {
  case gen, items {
    Seeded(_), [] -> #(Error(Nil), gen)
    Seeded(_), _ -> {
      let #(index, gen) = int(gen, 0, list.length(items) - 1)
      #(at(items, index), gen)
    }
    Mock([PickCall(expected, result), ..rest] as calls), _ ->
      case coerce(expected) == items {
        True -> #(coerce(result), Mock(rest))
        False -> mismatch(calls, pick_text(items))
      }
    Mock(calls), _ -> mismatch(calls, pick_text(items))
  }
}

/// Puts `items` in a random order.
pub fn shuffle(gen: Generator, items: List(a)) -> #(List(a), Generator) {
  case gen {
    Seeded(_) -> {
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
    Mock([ShuffleCall(expected, result), ..rest] as calls) ->
      case coerce(expected) == items {
        True -> #(coerce(result), Mock(rest))
        False -> mismatch(calls, shuffle_text(items))
      }
    Mock(calls) -> mismatch(calls, shuffle_text(items))
  }
}

fn mismatch(calls: List(Call), got: String) -> a {
  case calls {
    [] -> panic as { "random.mock: no call left for " <> got }
    [call, ..] ->
      panic as { "random.mock: expected " <> call_text(call) <> ", got " <> got }
  }
}

fn call_text(call: Call) -> String {
  case call {
    IntCall(from, to, _) -> int_text(from, to)
    FloatCall(_) -> "random.float()"
    PickCall(items, _) -> pick_text(coerce(items))
    ShuffleCall(items, _) -> shuffle_text(coerce(items))
  }
}

fn int_text(from: Int, to: Int) -> String {
  "random.int(" <> int.to_string(from) <> ", " <> int.to_string(to) <> ")"
}

fn pick_text(items: List(a)) -> String {
  "random.pick(" <> string.inspect(items) <> ")"
}

fn shuffle_text(items: List(a)) -> String {
  "random.shuffle(" <> string.inspect(items) <> ")"
}

fn at(items: List(a), index: Int) -> Result(a, Nil) {
  items |> list.drop(index) |> list.first
}

/// Returns `True` if `result` has the items of `items`, each one as many
/// times, `False` otherwise.
fn is_permutation(items: List(a), result: List(a)) -> Bool {
  case result {
    [] -> items == []
    [item, ..rest] ->
      case list.split_while(items, fn(x) { x != item }) {
        #(_, []) -> False
        #(before, [_, ..after]) ->
          is_permutation(list.append(before, after), rest)
      }
  }
}

/// The body is the mulberry32 of `random_step`, for the WebAssembly target.
/// Int has 64 bits there, so each operation keeps the low 32 of its result.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "random_step")
fn step(state: Int) -> #(Float, Int) {
  let #(high, state) = mulberry32(u32(state))
  let #(low, state) = mulberry32(state)
  let high = int.bitwise_shift_right(high, 5)
  let low = int.bitwise_shift_right(low, 6)
  #(int.to_float(high * 67_108_864 + low) /. 9_007_199_254_740_992.0, state)
}

/// The next 32 bits of mulberry32 from `state`, and the next state.
fn mulberry32(state: Int) -> #(Int, Int) {
  let state = u32(state + 0x6d2b79f5)
  let t = mul32(xor_shift(state, 15), int.bitwise_or(state, 1))
  let t =
    int.bitwise_exclusive_or(
      t,
      u32(t + mul32(xor_shift(t, 7), int.bitwise_or(t, 61))),
    )
  #(xor_shift(t, 14), state)
}

fn xor_shift(x: Int, bits: Int) -> Int {
  int.bitwise_exclusive_or(x, int.bitwise_shift_right(x, bits))
}

fn mul32(a: Int, b: Int) -> Int {
  u32(a * b)
}

fn u32(x: Int) -> Int {
  int.bitwise_and(x, 0xffffffff)
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "coerce")
fn coerce(value: a) -> b
