# Running a file

To run a Gleam file:

```sh
sgleam file.gleam
```

Sgleam looks for a `main`{.gleam} or `smain`{.gleam} function in the file. The `main`{.gleam} function takes no arguments:

```gleam
// hello.gleam
import gleam/io

pub fn main() {
  io.println("Hello world!")
}
```

```sh
$ sgleam hello.gleam
Hello world!
```

The `smain`{.gleam} function has three possible signatures. Without arguments, it works like `main`{.gleam}:

```gleam
// greeting.gleam
import gleam/io

pub fn smain() {
  io.println("Hello!")
}
```

```sh
$ sgleam greeting.gleam
Hello!
```

Receiving a `String`{.gleam}, the function receives all user input:

```gleam
// echo.gleam
import gleam/io

pub fn smain(input: String) {
  io.println("You typed: " <> input)
}
```

```sh
$ echo "test" | sgleam echo.gleam
You typed: test
```

Receiving a `List(String)`{.gleam}, the function receives the input split into lines:

```gleam
// count.gleam
import gleam/int
import gleam/io
import gleam/list

pub fn smain(lines: List(String)) {
  io.println("Lines: " <> int.to_string(list.length(lines)))
}
```

```sh
$ printf "a\nb\nc" | sgleam count.gleam
Lines: 3
```


# Interactive mode (REPL)

To enter interactive mode:

```sh
sgleam
```

In the REPL, you can type expressions, definitions (variables, functions, types), and commands:

```gleam-repl
> 1 + 2
3
> let x = 10
10
> x * 2
20
```

You can also load a file, making its definitions available in the REPL.
For example, given the file `double.gleam`:

```gleam
import sgleam/check

pub fn double(x: Int) -> Int {
  x * 2
}

pub fn double_examples() {
  check.eq(double(0), 0)
  check.eq(double(3), 6)
}
```

You can use the `double`{.gleam} function in the REPL:

```sh
sgleam repl double.gleam
```

```gleam-repl
> double(5)
10
> double(3) + 1
7
```

## REPL commands

`:quit` — Exits the REPL (or `Ctrl+d`).

`:type` — Shows the type of an expression without evaluating it:

```gleam-repl
> :type 1 + 2
Int
> :type [1, 2, 3]
List(Int)
```

`:debug` — Toggles debug mode, which shows the generated Gleam and JavaScript code before execution:

```gleam-repl
> :debug
Debug mode on.
> let x = 10
--- repl2_1.gleam ---
...
--- repl2_1.mjs ---
...
10
> :debug
Debug mode off.
```

## Imports in the REPL

Imports are supported and automatically merged:

```gleam-repl
> import gleam/int.{to_string}
> to_string(42)
"42"
> import gleam/int.{add}
> add(1, 2)
3
```


# Tests

To run the tests of a file:

```sh
sgleam test file.gleam
```

Tests are functions whose names end with `_examples` and use the `sgleam/check`{.gleam} module.

For example, given the file `test.gleam`:

```gleam
import sgleam/check

pub fn sum_examples() {
  check.eq(1 + 1, 2)
  check.eq(2 + 3, 5)
}

pub fn double_examples() {
  check.eq(2 * 0, 0)
  check.eq(2 * 3, 6)
  check.eq(2 * 4, 9)
}
```

```sh
sgleam test test.gleam
```

```
Running tests...
Failure at test.gleam (double_examples:11)
  Actual  : 8
  Expected: 9
5 tests, 4 success(es), 1 failure(s) and 0 error(s).
```

In this case, the test `check.eq(2 * 4, 9)`{.gleam} failed because `2 * 4`{.gleam} is `8`{.gleam}, not `9`{.gleam}.


# Formatting

To format source code:

```sh
sgleam format file.gleam
```

Or to format from standard input:

```sh
sgleam format < file.gleam
```


# Checking

To check that the code compiles correctly (type checking and syntax errors) without running it:

```sh
sgleam check file.gleam
```

If there are no errors, no output is produced. Otherwise, the errors are displayed.


# Commands

| Command | Description |
|---------|-------------|
| `sgleam [file]` | Run the file (shorthand for `sgleam run`) |
| `sgleam repl [file]` | Interactive mode (REPL) |
| `sgleam run file` | Run the file |
| `sgleam test file` | Run tests |
| `sgleam format [files]` | Format code (reads stdin if no files given) |
| `sgleam check file` | Check code (compile only) |
| `sgleam help` | Show help |

# Options

| Option | Description |
|--------|-------------|
| `-n` | Use Number instead of BigInt for integers |
| `-q` | Don't print welcome message in REPL |
| `--version` | Print version |
