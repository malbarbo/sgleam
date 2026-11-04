# Changelog

## 0.9.0 (2026-11-04)

- New `sgleam/mworld` library for multiplayer interactive programs, with `--players` to choose the number of players
- `smain` and the REPL run the `World`, `MultiWorld`, or `Animation` they return, so `world.run` is no longer needed
- `world.animate` replaced by `world.animation`, which returns an `Animation`
- Images, worlds, and animations shown in a window or in the terminal (kitty, sixel, or blocks), chosen with `-d`
- Keys moved from `sgleam/world` to the new `sgleam/key` module, and pad events in the new `sgleam/pad` module
- `gleam/io` and `sgleam/io` removed
- `image.same` and `check.same` to compare images, and a failed check shows its images
- Fixes to checks, shapes, and `scale_xy`
- Stack overflows show the Gleam file and line
- `smain` can take an `Env` with the command-line arguments, a random generator, and the time
- New `sgleam/random` library, with `random.mock` for tests
- `create_from`, `on_frame`, `on_frame_delta`, and `keys_as_pad` in `world` and `mworld`
- `Style` is a record, and the last style wins in `style.join`
- The version shows the git hash and the build date
- Updated to Gleam 1.19.2 and rquickjs 0.14.0

## 0.8.0 (2026-09-02)

- Ticks in `world` follow an absolute deadline instead of a countdown that drifted, and a render follows every state change, so `tick_rate` is now the frequency of `on_tick`
- Each REPL input gets a module of its own, and compile errors and warnings are relocated to what the user typed
- The REPL keeps reading after an unfinished expression or an attribute on its own line, runs a pending input when stdin ends, and indents by every kind of bracket
- An unknown REPL command is reported instead of compiled as Gleam, and commands are colored
- Add a `:time` command to measure how long an expression takes
- `let` with a string prefix pattern binds its names
- `as`, `echo`, and `opaque` in tab completion
- `sgleam test` prints the name of each test before it runs, and exits with a non-zero status when a check fails
- `sgleam format` reaches the `.gleam` files under a directory
- Paths in diagnostics and runtime errors drop the `src/` prefix, and a path that names no module is refused
- Updated to Gleam 1.18.1, stdlib 1.0.5, and rquickjs 0.12.2

## 0.7.0 (2026-04-01)

- Rewritten and improved image library with support for curves, all triangle variants, bitmap images, and dash styles
- Support for `let assert`, `let` with patterns, and `import` in the REPL
- Syntax highlighting, auto-indent, and tab completion in the REPL
- Input history with up/down arrows
- `:help` and `:theme` commands with One Light/Dark themes
- `sgleam/io.input` function for reading input
- CLI restructured with subcommands (use `sgleam --help`)
- Updated to Gleam 1.14 and stdlib 0.69.0

## 0.6.0 (2025-09-25)

- Add formatting in the web version
- Update Gleam to 1.11.1
- Add support for `smain` main functions
- Add library for images, animations, and interactive programs

## 0.5.1 (2025-04-25)

Changes of 0.5.0 and 0.5.1.

- Use the Gleam parser to analyze REPL statements
- Fix some errors in generic and recursive function definitions
- Add `:type` command
- Allow importing modules in non-interactive mode
- Allow opting in/out of big integers (`-n`)
- Allow interrupting execution with `ctrl-c`
- Update Gleam to 1.9.1 and stdlib to 0.57.0
- Add web version
- Fix file reading on Windows
