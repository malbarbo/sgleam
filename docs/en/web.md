The web playground allows you to use sgleam directly in the browser, without installing anything.


# Layout

The interface is divided into two panels:

- **Editor panel** (left or top): where you write Gleam code
- **REPL panel** (right or bottom): where the output is displayed

The initial layout is automatically chosen based on the screen dimensions: horizontal for wide screens and vertical for tall screens.


# Toolbar

- **Run** (▶): Formats the code, runs the tests, and loads definitions
- **Stop** (■): Stops execution
- **Theme** (☀): Toggles between light and dark themes
- **Layout**: Toggles between horizontal and vertical layout


# Keyboard shortcuts

| Shortcut | Description |
|----------|-------------|
| `Ctrl+r` | Run the definitions |
| `Ctrl+f` | Format the code |
| `Ctrl+j` | Focus the editor panel |
| `Ctrl+k` | Focus the REPL panel |
| `Ctrl+d` | Show/hide the editor panel |
| `Ctrl+i` | Show/hide the REPL panel |
| `Ctrl+l` | Toggle between horizontal and vertical layouts |
| `Ctrl+t` | Toggle between light and dark themes |
| `Ctrl+?` | Show the help window |
| `Esc` | Close the help window |


# How to use

1. Write your definitions in the editor panel
2. Press `Ctrl+r` or click **Run**
3. Use the REPL to evaluate expressions using the definitions

The **Run** button (or `Ctrl+r`) formats the code, runs the tests (`_examples`{.gleam} functions), and loads the definitions into the REPL. After that, you can call the functions defined in the editor directly in the REPL.

After the tests, the editor marks the code that no example ran, as `sgleam test` lists it in the terminal. The pointer over a mark shows the reason, such as a function that no example calls or a `case` clause that no example reaches. A mark on the right side of `a && b`{.gleam} means that `a`{.gleam} was never true, and a mark on the right side of `a || b`{.gleam} means that `a`{.gleam} was never false. The first edit takes the marks away, since a change can make any of them false. A program with no examples gets no marks.

Code that ran is not code that is right. A clause that only a failed example reaches still counts as run, and a helper function runs through the examples of the functions that call it, even with no examples of its own.

The REPL works like the command-line REPL: you can type expressions, definitions of variables, functions, and types.


# Themes

The playground supports two themes based on the Zed editor:

- **One Light** — light theme (default)
- **One Dark** — dark theme

The preference is saved in the browser.
