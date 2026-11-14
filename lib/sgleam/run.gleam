//// How a program runs on the WebAssembly target. The compilation of the
//// user's package makes `start` run each `main`.

import gleam/int
import gleam/list
import gleam/option
import gleam/wasm/failure.{type Asserted, type Failure}
import sgleam/debug
import sgleam/wasi

/// Runs `main`. A failure that `main` does not catch is printed as on the
/// JavaScript target, and the program exits with 1.
pub fn start(main: fn() -> a) -> Nil {
  case failure.catch(main) {
    Ok(_) -> Nil
    Error(failure) -> {
      show(failure)
      wasi.exit(1)
    }
  }
}

fn show(failure: Failure) -> Nil {
  // The path of a module starts at the root of the package, and the user
  // gave the path from `src`.
  let file = case failure.file {
    "src/" <> file -> file
    file -> file
  }
  let line = int.to_string(failure.line)
  case failure.kind {
    // The JavaScript target shows the calls that repeat, and the WebAssembly
    // target only knows the call that went past the limit.
    failure.StackOverflow -> {
      wasi.println("Stack overflow")
      wasi.println(
        "  at " <> failure.function <> " (" <> file <> ":" <> line <> ")",
      )
      wasi.println("  ...")
    }
    _ -> {
      let message =
        option.lazy_unwrap(failure.message, fn() {
          failure.default_message(failure.kind)
        })
      wasi.println(
        "Error at " <> file <> " (" <> failure.function <> ":" <> line <> ")",
      )
      wasi.println("  " <> message)
      list.each(details(failure), fn(detail) {
        wasi.println("  " <> detail.0 <> ": " <> detail.1)
      })
    }
  }
}

/// The values that explain `failure`, by name, as the JavaScript target
/// shows them after the message.
pub fn details(failure: Failure) -> List(#(String, String)) {
  case failure.details {
    failure.LetAssertValue(value) -> [#("value", debug.show(value))]
    failure.BinaryOperator(operator:, left:, right:) -> [
      #("operator", operator),
      #("left", asserted(left)),
      #("right", asserted(right)),
    ]
    failure.FunctionCall(arguments) ->
      list.index_map(arguments, fn(argument, i) {
        #("argument " <> int.to_string(i + 1), asserted(argument))
      })
    // The value of a plain expression is always `False`.
    failure.Expression(_) | failure.NoDetails -> []
  }
}

fn asserted(expression: Asserted) -> String {
  case expression {
    failure.Evaluated(value:, ..) -> debug.show(value)
    failure.Unevaluated -> "not evaluated"
  }
}
