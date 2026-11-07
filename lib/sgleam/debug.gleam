//// What `echo` prints on the WebAssembly target. The compilation of the
//// user's package calls `print` for each `echo`.

import gleam/dynamic.{type Dynamic}
import gleam/int
import gleam/option.{type Option, None, Some}
import gleam/wasm/reflect
import sgleam/wasi

/// Prints the place of an `echo`, its message, and `value` on stdout, as
/// the JavaScript target prints them.
pub fn print(
  file: String,
  line: Int,
  message: Option(String),
  value: Dynamic,
) -> Nil {
  // The path of a module starts at the root of the package, and the user
  // gave the path from `src`.
  let file = case file {
    "src/" <> file -> file
    file -> file
  }
  let message = case message {
    Some(message) -> " " <> message
    None -> ""
  }
  wasi.println(file <> ":" <> int.to_string(line) <> message)
  wasi.println(reflect.inspect(value))
}
