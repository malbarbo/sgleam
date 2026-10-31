# Running a file

To run a Gleam file:

```sh
sgleam file.gleam
```

Sgleam looks for a `smain`{.gleam} or `main`{.gleam} function in the file. Sgleam prints what `smain`{.gleam} returns. A `String`{.gleam} prints as it is, an image shows as in the REPL, a world or an animation runs, and any other value prints as Gleam writes it:

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

Receiving an `Env`{.gleam}, from the `sgleam/env`{.gleam} module, the function receives what the program knows when it starts. `env.args`{.gleam} gives the arguments after the file, `env.random`{.gleam} gives a random generator, and `env.time`{.gleam} gives the milliseconds since 1970-01-01 00:00 UTC:

```gleam
// hello.gleam
import gleam/string
import sgleam/env.{type Env}

pub fn smain(env: Env) {
  "Hello, " <> string.join(env.args(env), " and ") <> "!"
}
```

```sh
$ sgleam hello.gleam Ana Bia
Hello, Ana and Bia!
```

An argument that starts with `-` goes after `--`, as in `sgleam run hello.gleam -- -x`. Otherwise sgleam reads it as one of its options.

The `main`{.gleam} function takes no arguments, and sgleam does not print what it returns.

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


# Random numbers

The `sgleam/random`{.gleam} module draws numbers from a generator. A generator is a value, so the same generator always gives the same number. Each function returns the number together with the next generator, and the next draw uses the new one:

```gleam
// dice.gleam
import sgleam/env.{type Env}
import sgleam/random

pub fn smain(env: Env) {
  let gen = env.random(env)
  let #(a, gen) = random.int(gen, 1, 6)
  let #(b, _) = random.int(gen, 1, 6)
  #(a, b)
}
```

`env.random`{.gleam} gives a generator with a different seed each run. `random.new(seed)`{.gleam} creates one with a fixed seed, and the same seed gives the same sequence. `random.int(gen, from, to)`{.gleam} draws an `Int`{.gleam} between `from` and `to`, both included. `random.float(gen)`{.gleam} draws a `Float`{.gleam} from 0.0 up to 1.0, excluded. `random.pick(gen, items)`{.gleam} draws an item of a list, and `random.shuffle(gen, items)`{.gleam} puts a list in a random order.

A test gives the program a generator with known answers. `random.mock`{.gleam} takes the calls the program makes, in order, each with the result it returns:

```gleam
import sgleam/check
import sgleam/random.{type Generator}

pub fn roll(gen: Generator) -> #(Int, Generator) {
  let #(a, gen) = random.int(gen, 1, 6)
  let #(b, gen) = random.int(gen, 1, 6)
  #(a + b, gen)
}

pub fn roll_examples() {
  let gen =
    random.mock([random.int_returns(1, 6, 2), random.int_returns(1, 6, 5)])
  check.eq(roll(gen).0, 7)
}
```

`random.float_returns`{.gleam}, `random.pick_returns`{.gleam} and `random.shuffle_returns`{.gleam} describe the other calls. Each draw has to match the first call of the list, with the same arguments. A different call, or a draw after the list ends, stops the program with a message that says what the mock expected. For a function that takes an `Env`{.gleam}, `env.fake()`{.gleam} gives an env with no arguments, `random.new(0)`{.gleam} and time 0, and `env.with_args`{.gleam}, `env.with_random`{.gleam} and `env.with_time`{.gleam} change it:

```gleam
env.fake() |> env.with_random(random.mock([random.int_returns(1, 6, 2)]))
```


# Images and worlds

A world, or an animation, shows its frames in a window, and closing the window ends the program. When no window opens, as over ssh, the frames show in the terminal, and `Ctrl+c` ends the program. If the terminal shows no images either, each frame prints as `Image`. The REPL shows an image in the terminal, or prints `Image` when the terminal shows no images.

While a world shows in the terminal, what the program prints, such as an `echo`{.gleam}, goes to the five lines under the frame. When the world ends, the last frame stays on the screen with those lines below it.

The `-d` option chooses where images show. `-d window` shows the frames of a world only in a window, and the REPL still shows its images in the terminal. `-d kitty`, `-d sixel` and `-d blocks` use only the terminal, through the graphics of Kitty, through Sixel, or through colored blocks, which any terminal with true color shows. With `-d kitty` and `-d sixel`, sgleam asks the terminal first, and stops with an error when the terminal does not support the mode. `-d blocks` does not ask, so a terminal without true color shows wrong colors. `-d none` shows no image and prints `Image` in its place.

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
| `sgleam run file [args]` | Run the file, with arguments for `smain(env)` |
| `sgleam test file` | Run tests |
| `sgleam format [files]` | Format code (reads stdin if no files given) |
| `sgleam check file` | Check code (compile only) |
| `sgleam help [command]` | Show help, of the program or of a command |

# Options

| Option | Description |
|--------|-------------|
| `-n` | Use Number instead of BigInt for integers |
| `-d MODE` | Show images in `window`, `kitty`, `sixel`, `blocks` or `none` |
| `--players N` | Play a multiplayer world with N players, each in a window |
| `-q` | Don't print welcome message in REPL |
| `--version` | Print version |
