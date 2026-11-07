import gleam/dynamic
import gleam/dynamic/decode
import gleam/int
import gleam/list
import gleam/option
import gleam/result
import gleam/wasm/failure.{type Failure}
import gleam/wasm/global
import sgleam/image.{type Image}
import sgleam/wasi

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_equal")
pub fn eq(a: x, b: x) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_approx")
pub fn approx(a: Float, b: Float, tolerance: Float) -> Bool

/// Passes if `a` and `b` are the same by `image.same`, which compares the
/// pixels when the trees differ.
@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_same")
pub fn same(a: Image, b: Image) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_true")
pub fn true(val: Bool) -> Bool

@external(javascript, "../sgleam/sgleam_ffi.mjs", "check_false")
pub fn false(val: Bool) -> Bool

/// The test that `run_tests` is running: the file and the function.
const test_file = "sgleam/check/file"

const test_function = "sgleam/check/function"

const successes = "sgleam/check/successes"

const failures = "sgleam/check/failures"

const errors = "sgleam/check/errors"

/// Runs `assertion`, an `assert` of the user's code, on the WebAssembly
/// target. Inside a test, a true `assert` is a success, a false one is a
/// failure, and a `panic` in its expression is an error. The test goes on to
/// the next `assert` in the three cases. Outside a test, a false `assert`
/// stops the program, as on the JavaScript target.
pub fn run(assertion: fn() -> Nil) -> Nil {
  case current() {
    Error(Nil) -> assertion()
    Ok(running) ->
      case failure.catch(assertion) {
        Ok(Nil) -> count(successes)
        Error(failure) if failure.kind == failure.Assert -> {
          show("Failure", failure, running)
          count(failures)
        }
        Error(failure) -> {
          show("Error", failure, running)
          count(errors)
        }
      }
  }
}

/// Runs each test, a file, the name of a function and the function, as
/// `sgleam test` runs the `_examples` functions on the JavaScript target.
/// A failure in the function outside an `assert` is an error, and ends the
/// test. Returns `True` if every test passed, `False` otherwise. It prints
/// nothing when there are no tests.
pub fn run_tests(tests: List(#(String, String, fn() -> Nil))) -> Bool {
  case tests {
    [] -> True
    _ -> {
      // This prints before the test runs, so a test that hangs still says
      // where.
      wasi.println("Running tests...")
      list.each(tests, fn(test_) {
        let #(file, function, run) = test_
        wasi.println("  " <> file <> "/" <> function)
        global.set(test_file, dynamic.string(file))
        global.set(test_function, dynamic.string(function))
        case failure.catch(run) {
          Ok(Nil) -> Nil
          Error(failure) -> {
            show("Error", failure, #(file, function))
            count(errors)
          }
        }
      })
      global.set(test_file, dynamic.nil())
      let successes = counted(successes)
      let failures = counted(failures)
      let errors = counted(errors)
      wasi.println(
        plural(successes + failures + errors, "test", "tests")
        <> ", "
        <> plural(successes, "success", "successes")
        <> ", "
        <> plural(failures, "failure", "failures")
        <> " and "
        <> plural(errors, "error", "errors")
        <> ".",
      )
      failures + errors == 0
    }
  }
}

/// The file and the function of the test that is running.
fn current() -> Result(#(String, String), Nil) {
  let string = fn(key) {
    global.get(key)
    |> result.try(fn(value) {
      decode.run(value, decode.string) |> result.replace_error(Nil)
    })
  }
  use file <- result.try(string(test_file))
  use function <- result.map(string(test_function))
  #(file, function)
}

fn count(key: String) -> Nil {
  global.set(key, dynamic.int(counted(key) + 1))
}

fn counted(key: String) -> Int {
  global.get(key)
  |> result.try(fn(value) {
    decode.run(value, decode.int) |> result.replace_error(Nil)
  })
  |> result.unwrap(0)
}

/// Shows `failure` under the name of `running`, as `kind at` its location.
fn show(kind: String, failure: Failure, running: #(String, String)) -> Nil {
  let message =
    option.lazy_unwrap(failure.message, fn() {
      failure.default_message(failure.kind)
    })
  wasi.println("    " <> kind <> " at " <> location(failure, running))
  wasi.println("      " <> message)
}

/// Where `failure` is, with only what the name of `running` does not already
/// say.
fn location(failure: Failure, running: #(String, String)) -> String {
  let #(test_file, test_function) = running
  // The path of a module starts at the root of the package, and the user
  // gave the path from `src`.
  let file = case failure.file {
    "src/" <> file -> file
    file -> file
  }
  let line = int.to_string(failure.line)
  case file == test_file, failure.function == test_function {
    True, True -> "line " <> line
    True, False -> failure.function <> ":" <> line
    False, _ -> file <> " (" <> failure.function <> ":" <> line <> ")"
  }
}

fn plural(n: Int, one: String, many: String) -> String {
  int.to_string(n)
  <> " "
  <> case n {
    1 -> one
    _ -> many
  }
}
