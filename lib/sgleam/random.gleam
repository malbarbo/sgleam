import gleam/dynamic.{type Dynamic}
import gleam/float
import gleam/int
import gleam/list
import gleam/result
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
  PickCall(items: Dynamic, index: Result(Int, Nil))
  ShuffleCall(items: Dynamic, order: List(Int))
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
  let index = case result, items {
    Error(Nil), [] -> Ok(Error(Nil))
    Error(Nil), _ -> Error(Nil)
    Ok(item), _ -> index_of(items, item, 0) |> result.map(Ok)
  }
  case index {
    Ok(index) -> PickCall(coerce(items), index)
    Error(Nil) ->
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
  case
    permutation(
      list.index_map(items, fn(item, index) { #(index, item) }),
      result,
    )
  {
    Ok(order) -> ShuffleCall(coerce(items), order)
    Error(Nil) ->
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
    Mock(calls) ->
      mismatch(
        calls,
        "random.int(" <> int.to_string(from) <> ", " <> int.to_string(to) <> ")",
      )
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
    Mock([PickCall(expected, index), ..rest] as calls), _ ->
      case coerce(expected) == items {
        True -> #(result.try(index, at(items, _)), Mock(rest))
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
    Mock([ShuffleCall(expected, order), ..rest] as calls) ->
      case coerce(expected) == items {
        True -> #(
          list.filter_map(order, fn(index) { at(items, index) }),
          Mock(rest),
        )
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
    IntCall(from, to, _) ->
      "random.int(" <> int.to_string(from) <> ", " <> int.to_string(to) <> ")"
    FloatCall(_) -> "random.float()"
    PickCall(items, _) -> pick_text(coerce(items))
    ShuffleCall(items, _) -> shuffle_text(coerce(items))
  }
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

fn index_of(items: List(a), item: a, index: Int) -> Result(Int, Nil) {
  case items {
    [] -> Error(Nil)
    [first, ..] if first == item -> Ok(index)
    [_, ..rest] -> index_of(rest, item, index + 1)
  }
}

/// The indices in `available` of the items of `result`, each index used once.
fn permutation(
  available: List(#(Int, a)),
  result: List(a),
) -> Result(List(Int), Nil) {
  case result {
    [] if available == [] -> Ok([])
    [] -> Error(Nil)
    [item, ..rest] -> {
      use #(index, available) <- result.try(take(available, item))
      use order <- result.map(permutation(available, rest))
      [index, ..order]
    }
  }
}

/// The index of the first pair with `item`, and `available` without it.
fn take(
  available: List(#(Int, a)),
  item: a,
) -> Result(#(Int, List(#(Int, a))), Nil) {
  case available {
    [] -> Error(Nil)
    [#(index, first), ..rest] if first == item -> Ok(#(index, rest))
    [pair, ..rest] -> {
      use #(index, rest) <- result.map(take(rest, item))
      #(index, [pair, ..rest])
    }
  }
}

@external(javascript, "../sgleam/sgleam_ffi.mjs", "random_step")
fn step(state: Int) -> #(Float, Int)

@external(javascript, "../sgleam/sgleam_ffi.mjs", "coerce")
fn coerce(value: a) -> b
