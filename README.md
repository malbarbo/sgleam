# sgleam

A student-friendly Gleam environment with an interactive REPL, image support,
and interactive programs, designed for teaching functional programming.

sgleam is built on [Gleam](https://gleam.run/) for compilation and
[QuickJS](https://bellard.org/quickjs/) (via
[rquickjs](https://github.com/aspect-build/rquickjs)) for execution. A similar
project for Python is [spython](https://github.com/malbarbo/spython).

## Features

- **Gleam stdlib included** — 18 standard library modules available
  out-of-the-box, no project setup required
- **Scripting** — run single `.gleam` files directly, without creating a
  Gleam project. Use `smain` to read from stdin, making sgleam suitable for
  solving [Advent of Code](https://adventofcode.com/) challenges and other
  scripting tasks
- **Interactive REPL** — auto-indent, tab completion, multiline editing
- **BigInt / Number modes** — BigInt (default) for arbitrary-precision
  integers, or Number mode (`-n`) for better performance
- **Image library** — built-in SVG-based graphics library for teaching,
  inspired by [HtDP](https://htdp.org/) image teachpacks
- **Interactive programs** — `world` library for animations and
  keyboard-driven programs
- **Testing** — `check` library for example-based testing
- **WASM support** — runs in the browser via a WASM build

## Build

Requires Rust (stable). Dependencies are fetched automatically.

```bash
cargo build                # debug build
cargo build --release      # optimized build
```

## Usage

```bash
# Start the REPL
sgleam

# Run a script
sgleam file.gleam

# Run with Number mode (instead of BigInt)
sgleam -n file.gleam
```

### Scripting with `smain`

sgleam can run single `.gleam` files without a project. Define a `smain`
function to read input from stdin:

```gleam
import gleam/io
import gleam/list
import gleam/string

/// Reads lines from stdin and prints them reversed
pub fn smain(lines: List(String)) {
  lines
  |> list.map(string.reverse)
  |> list.each(io.println)
}
```

```bash
echo -e "hello\nworld" | sgleam file.gleam
# olleh
# dlrow
```

The `smain` function accepts three signatures:

- `fn smain() -> a` — no input
- `fn smain(input: String) -> a` — all stdin as a single string
- `fn smain(lines: List(String)) -> a` — stdin split into lines

## Image Library

sgleam includes a built-in image library for teaching graphics programming,
inspired by the [HtDP](https://htdp.org/) image teachpacks. Images are
rendered as SVG.

```gleam
import sgleam/image.{circle, overlay, rectangle, to_svg}
import sgleam/fill
import sgleam/stroke

let img = overlay(circle(30, stroke.red), rectangle(80, 50, fill.blue))
```

The library provides:

- **Shapes** — rectangles, circles, ellipses, triangles (7 constructors),
  polygons, stars, lines, wedges, bezier curves
- **Transformations** — rotate, scale, flip, crop
- **Composition** — overlay, underlay, beside, above (with alignment options)
- **Scenes** — place images at coordinates, draw lines/polygons/curves
- **Text and fonts** — text rendering with customizable fonts
- **Styles** — fill and stroke with color, opacity, width, dash patterns
- **Colors** — 140+ CSS named colors, `rgb()`, `rgba()`
- **Animation** — `world` library for interactive programs with keyboard input

## Architecture

The workspace has three crates:

- `engine` — shared library (REPL engine, Gleam compiler integration, QuickJS
  runtime)
- `cli` — CLI binary with REPL
- `wasm` — WASM target for browser use

## Development

```bash
make check              # clippy + fmt + deno fmt
make test               # cargo test + deno test (WASM)
make test-rs            # cargo test only (faster)
make wasm               # build WASM binary
```

## License

Apache 2.0 — see [LICENSE](LICENSE).
