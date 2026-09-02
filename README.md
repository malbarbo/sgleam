# sgleam

sgleam is a version of [Gleam](https://gleam.run/) for students. It has a
REPL, runs a single `.gleam` file without a project, and comes with libraries
for drawing images and writing interactive programs, in the spirit of
[HtDP](https://htdp.org/).

You can try it in the browser at <https://malbarbo.pro.br/sgleam/>. The web
version has an editor, the REPL, and shows images and interactive programs
inline.

sgleam uses the Gleam compiler and runs the generated JavaScript on
[QuickJS](https://bellard.org/quickjs/), through
[rquickjs](https://github.com/aspect-build/rquickjs). There is a similar
project for Python, [spython](https://github.com/malbarbo/spython).

## Install

Download the archive for your system from the
[releases page](https://github.com/malbarbo/sgleam/releases), extract it, and
put the `sgleam` binary somewhere on your `PATH`. That is all. The Gleam
standard library ships inside the binary.

To build from source you need stable Rust:

```sh
cargo build --release
```

## Use

`sgleam` with no arguments starts the REPL. It highlights the code as you type,
completes names with tab, and keeps reading when an expression is not finished.

To run a program, give it the file:

```sh
sgleam hello.gleam
```

sgleam calls the `main` function of the file. A program that reads from stdin
defines `smain` instead, and gets the input as one `String` or as a
`List(String)` of lines:

```gleam
import gleam/io
import gleam/list
import gleam/string

pub fn smain(lines: List(String)) {
  lines
  |> list.map(string.reverse)
  |> list.each(io.println)
}
```

```sh
$ printf 'hello\nworld\n' | sgleam reverse.gleam
olleh
dlrow
```

Integers have arbitrary precision. The `-n` flag uses JavaScript numbers
instead, which is faster.

Tests are functions whose names end in `_examples`. They state what a function
returns for a few inputs, with the `sgleam/check` module, and `sgleam test`
runs them:

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

`sgleam format` formats a file, and `sgleam repl file.gleam` loads the
definitions of a file into the REPL. `sgleam --help` lists the rest.

## Images and interactive programs

The `sgleam/image` module builds a picture out of shapes, places pictures over,
beside, and above each other, and rotates, scales, and crops them. A picture is
an SVG.

```gleam
import sgleam/fill
import sgleam/image
import sgleam/stroke

image.overlay(image.circle(30, stroke.red), image.rectangle(80, 50, fill.blue))
```

The `sgleam/world` module is for animations and games. A program is a state, a
function that draws the state, and functions that update the state on each
tick of the clock or key press:

```gleam
world.create(initial_state, draw)
|> world.on_tick(tick)
|> world.on_key_press(key)
|> world.run()
```

Both work best in the web version, which shows the picture as you go. The
[examples](examples/) directory has a few complete programs.

## Documentation

The guides are in [English](docs/en/) and in
[Portuguese](docs/pt-br/). There is one for the command line and one for the
web version, which also covers images and interactive programs.

## Development

The workspace has three crates. `engine` has the compiler integration, the
QuickJS runtime, and the REPL. `cli` is the binary, and `wasm` is the build
for the browser.

```sh
make check      # clippy, cargo fmt, deno fmt
make test       # cargo test and the WASM tests
make test-rs    # cargo test only
make wasm       # the WASM binary
```

## License

Apache 2.0. See [LICENSE](LICENSE).
