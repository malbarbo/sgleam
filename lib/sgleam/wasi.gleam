//// What a program on the WebAssembly target reads from WASI when it starts:
//// the arguments, a seed, the time and stdin. The run calls these
//// functions, and a program does not.
////
//// A call that WASI fails gives what an empty source gives: no arguments, a
//// seed of 0, the time 0 and no lines.

import gleam/bit_array
import gleam/list
import gleam/result
import gleam/string
import gleam/wasm.{type I32, type I64}
import gleam/wasm/memory

const buffer_size = 65_536

const line_feed = 10

const carriage_return = 13

/// The arguments after the program.
pub fn args() -> List(String) {
  let text = {
    use <- memory.scope
    use sizes <- result.try(memory.alloc(8))
    use _ <- result.try(succeeded(args_sizes_get(i32(sizes), i32(sizes + 4))))
    use count <- result.try(memory.load_u32(sizes))
    use size <- result.try(memory.load_u32(sizes + 4))
    use pointers <- result.try(memory.alloc(4 * count))
    use buffer <- result.try(memory.alloc(size))
    use _ <- result.try(succeeded(args_get(i32(pointers), i32(buffer))))
    use bytes <- result.try(memory.read(buffer, size))
    bit_array.to_string(bytes)
  }
  case text {
    // Each argument ends in a 0, the name of the program too.
    Ok(text) ->
      text |> string.drop_end(1) |> string.split("\u{0}") |> list.drop(1)
    Error(Nil) -> []
  }
}

/// A number from 0 to 2^32 - 1 that changes from one run to the next.
pub fn seed() -> Int {
  {
    use <- memory.scope
    use address <- result.try(memory.alloc(4))
    use _ <- result.try(succeeded(random_get(i32(address), i32(4))))
    memory.load_u32(address)
  }
  |> result.unwrap(0)
}

/// The milliseconds since 1970-01-01 00:00 UTC.
pub fn time() -> Int {
  {
    use <- memory.scope
    use address <- result.try(memory.alloc(8))
    let realtime = i32(0)
    let precision = wasm.i64_from_int(1_000_000)
    use _ <- result.try(
      succeeded(clock_time_get(realtime, precision, i32(address))),
    )
    memory.load_i64(address)
  }
  |> result.map(fn(nanoseconds) { nanoseconds / 1_000_000 })
  |> result.unwrap(0)
}

/// The lines of stdin, read to its end, without the `\n` or `\r\n` that
/// ends each one. The lines stop before the first one that is not UTF-8.
pub fn stdin_lines() -> List(String) {
  let bytes = {
    use <- memory.scope
    // An iovec, the address and the size of the buffer, then the size read.
    use iovec <- result.try(memory.alloc(12))
    use buffer <- result.try(memory.alloc(buffer_size))
    use _ <- result.try(memory.store_u32(iovec, buffer))
    use _ <- result.try(memory.store_u32(iovec + 4, buffer_size))
    Ok(read_stdin(iovec, buffer, []))
  }
  let bytes = result.unwrap(bytes, <<>>)
  split_lines(bytes, bytes, 0, 0, [])
}

fn read_stdin(iovec: Int, buffer: Int, chunks: List(BitArray)) -> BitArray {
  let chunk = {
    let stdin = i32(0)
    use _ <- result.try(
      succeeded(fd_read(stdin, i32(iovec), i32(1), i32(iovec + 8))),
    )
    use size <- result.try(memory.load_u32(iovec + 8))
    memory.read(buffer, size)
  }
  case chunk {
    Ok(chunk) if chunk != <<>> -> read_stdin(iovec, buffer, [chunk, ..chunks])
    _ -> bit_array.concat(list.reverse(chunks))
  }
}

/// The lines of `bytes`, of which `rest` is what is left from `at` on. The
/// line that is being read starts at `start`.
fn split_lines(
  bytes: BitArray,
  rest: BitArray,
  start: Int,
  at: Int,
  lines: List(String),
) -> List(String) {
  case rest {
    <<byte, rest:bytes>> if byte == line_feed ->
      case line(bytes, start, at) {
        Ok(line) -> split_lines(bytes, rest, at + 1, at + 1, [line, ..lines])
        Error(Nil) -> list.reverse(lines)
      }
    <<_, rest:bytes>> -> split_lines(bytes, rest, start, at + 1, lines)
    // The text either ends in a line feed, or has a last line without one.
    _ ->
      case at == start, line(bytes, start, at) {
        False, Ok(line) -> list.reverse([line, ..lines])
        _, _ -> list.reverse(lines)
      }
  }
}

/// The line of `bytes` from `start` to `end`, without a `\r` at its end.
fn line(bytes: BitArray, start: Int, end: Int) -> Result(String, Nil) {
  let end = case bit_array.slice(bytes, end - 1, 1) {
    Ok(<<byte>>) if byte == carriage_return && end > start -> end - 1
    _ -> end
  }
  use line <- result.try(bit_array.slice(bytes, start, end - start))
  bit_array.to_string(line)
}

fn i32(n: Int) -> I32 {
  wasm.i32_wrap(n)
}

/// Returns `Ok(Nil)` if the errno of WASI is 0, `Error(Nil)` otherwise.
fn succeeded(errno: I32) -> Result(Nil, Nil) {
  case wasm.i32_to_int(errno) {
    0 -> Ok(Nil)
    _ -> Error(Nil)
  }
}

@external(webassembly, "wasi_snapshot_preview1", "args_sizes_get")
fn args_sizes_get(_count: I32, _size: I32) -> I32 {
  panic as "only on the WebAssembly target"
}

@external(webassembly, "wasi_snapshot_preview1", "args_get")
fn args_get(_pointers: I32, _buffer: I32) -> I32 {
  panic as "only on the WebAssembly target"
}

@external(webassembly, "wasi_snapshot_preview1", "random_get")
fn random_get(_buffer: I32, _size: I32) -> I32 {
  panic as "only on the WebAssembly target"
}

@external(webassembly, "wasi_snapshot_preview1", "clock_time_get")
fn clock_time_get(_clock: I32, _precision: I64, _time: I32) -> I32 {
  panic as "only on the WebAssembly target"
}

@external(webassembly, "wasi_snapshot_preview1", "fd_read")
fn fd_read(_fd: I32, _iovecs: I32, _count: I32, _size_read: I32) -> I32 {
  panic as "only on the WebAssembly target"
}
