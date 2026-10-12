# Running a file

To run a Gleam file:

```sh
sgleam file.gleam
```

Sgleam looks for a `smain`{.gleam} or `main`{.gleam} function in the file. Sgleam prints what `smain`{.gleam} returns. A `String`{.gleam} prints as it is, and any other value prints as Gleam writes it:

```gleam
// hello.gleam
pub fn smain() {
  "Hello world!"
}
```

```sh
$ sgleam hello.gleam
Hello world!
```

The `smain`{.gleam} function may also take the input of the user. Receiving a `String`{.gleam}, the function receives all of the input:

```gleam
// echo.gleam
pub fn smain(input: String) {
  "You typed: " <> input
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
import gleam/list

pub fn smain(lines: List(String)) {
  "Lines: " <> int.to_string(list.length(lines))
}
```

```sh
$ printf "a\nb\nc" | sgleam count.gleam
Lines: 3
```

The `main`{.gleam} function takes no arguments, and sgleam does not print what it returns. It fits a program that shows images or runs a world.

To see a value in the middle of a program, use `echo`{.gleam}. It prints the file and the line, then the value:

```gleam
// double.gleam
pub fn smain() {
  let x = echo 2 * 3
  x * 2
}
```

```sh
$ sgleam double.gleam
double.gleam:3
6
12
```


# Images and worlds

A world, or an animation, shows its frames in a window, and closing the window ends the program. When no window opens, as over ssh, the frames show in the terminal, and `Ctrl+c` ends the program. If the terminal shows no images either, each frame prints as `Image`. The REPL shows an image in the terminal, or prints `Image` when the terminal shows no images.

While a world shows in the terminal, what the program prints, such as an `echo`{.gleam}, goes to the five lines under the frame. When the world ends, the last frame stays on the screen with those lines below it.

The `-d` option chooses where images show. `-d window` shows the frames of a world only in a window, and the REPL still shows its images in the terminal. `-d kitty`, `-d sixel` and `-d blocks` use only the terminal, through the graphics of Kitty, through Sixel, or through colored blocks, which any terminal with true color shows. `-d none` shows no image and prints `Image` in its place.

```sh
sgleam -d blocks bounce.gleam
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
--- repl1.gleam ---
...
--- repl1_1.gleam ---
...
--- repl1_1.mjs ---
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
  test.gleam/double_examples
    Failure at line 11
      Actual  : 8
      Expected: 9
  test.gleam/sum_examples
5 tests, 4 successes, 1 failure and 0 errors.
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
| `sgleam help [command]` | Show help, of the program or of a command |

# Options

| Option | Description |
|--------|-------------|
| `-n` | Use Number instead of BigInt for integers |
| `-d MODE` | Show images in `window`, `kitty`, `sixel`, `blocks` or `none` |
| `-q` | Don't print welcome message in REPL |
| `--version` | Print version |
