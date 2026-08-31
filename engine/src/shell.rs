//! The `:` commands, over a repl that knows none of them.

use std::time::Duration;

use crate::{
    engine::Engine,
    parser,
    repl::{Failed, Repl},
};

pub fn welcome_message() -> String {
    format!(
        "Welcome to {}.\nType ctrl-d or \":quit\" to exit.\n",
        crate::version()
    )
}

/// What became of an input. `repl_run` answers with these numbers.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Ok = 0,
    Error = 1,
    Quit = 2,
}

pub struct Shell<E: Engine> {
    repl: Repl<E>,
    commands: Vec<Command>,
}

struct Command {
    name: &'static str,
    arg: Arg,
    help: &'static str,
    action: Action,
}

#[derive(Clone, Copy)]
enum Arg {
    None,
    /// Gleam, which is what says whether the input is finished.
    Expr,
    /// An optional word, which the help shows exactly as it stands here.
    Word(&'static str),
}

enum Action {
    Builtin(Builtin),
    Host(HostAction),
}

type HostAction = Box<dyn FnMut(&str) -> Result<(), String>>;

#[derive(Clone, Copy)]
enum Builtin {
    Help,
    Quit,
    Type,
    Time,
    Debug,
}

const BUILTINS: &[(&str, Arg, &str, Builtin)] = &[
    (":help", Arg::None, "Show this help", Builtin::Help),
    (":quit", Arg::None, "Exit the REPL", Builtin::Quit),
    (
        ":type",
        Arg::Expr,
        "Show the type of an expression without running it",
        Builtin::Type,
    ),
    (
        ":time",
        Arg::Expr,
        "Run an expression and show how long it took",
        Builtin::Time,
    ),
    (":debug", Arg::None, "Toggle debug mode", Builtin::Debug),
];

/// The keywords that are the whole of what they say: `todo` on its own, `Ok`
/// with what it wraps stuck to it.
const KEYWORDS: &[&str] = &["Error", "False", "Nil", "Ok", "True", "fn", "panic", "todo"];

/// The keywords that never end an input, and so the ones the completion hands
/// back with the space already typed, as it does for `:type`.
const KEYWORDS_OPEN: &[&str] = &[
    "as", "assert", "case", "const", "echo", "import", "let", "opaque", "pub", "type", "use",
];

impl<E: Engine> Shell<E> {
    pub fn new(repl: Repl<E>) -> Shell<E> {
        let commands = BUILTINS
            .iter()
            .map(|&(name, arg, help, builtin)| Command {
                name,
                arg,
                help,
                action: Action::Builtin(builtin),
            })
            .collect();
        Shell { repl, commands }
    }

    /// A command of the host's. `word` is what the help shows after the name,
    /// for one that takes a word; `run` gets what followed the name, and
    /// answers with what to tell the user when it refuses.
    pub fn add(
        &mut self,
        name: &'static str,
        word: Option<&'static str>,
        help: &'static str,
        run: impl FnMut(&str) -> Result<(), String> + 'static,
    ) {
        debug_assert!(self.commands.iter().all(|c| c.name != name));
        self.commands.push(Command {
            name,
            arg: word.map_or(Arg::None, Arg::Word),
            help,
            action: Action::Host(Box::new(run)),
        });
    }

    pub fn run(&mut self, input: &str) -> Status {
        let Some((name, arg)) = split(input) else {
            return status(self.repl.run(input));
        };
        let Some(command) = self.commands.iter_mut().find(|c| c.name == name) else {
            println!("Unknown command {name}. Type :help to see the commands.");
            return Status::Error;
        };
        match (command.arg, arg.is_empty()) {
            (Arg::None, false) => {
                println!("The {name} command takes nothing after it.");
                return Status::Error;
            }
            (Arg::Expr, true) => {
                println!("The {name} command takes an expression.");
                return Status::Error;
            }
            _ => {}
        }
        match &mut command.action {
            Action::Host(run) => match run(arg) {
                Ok(()) => Status::Ok,
                Err(message) => {
                    println!("{message}");
                    Status::Error
                }
            },
            Action::Builtin(builtin) => {
                let builtin = *builtin;
                self.builtin(builtin, arg)
            }
        }
    }

    fn builtin(&mut self, builtin: Builtin, arg: &str) -> Status {
        match builtin {
            Builtin::Help => {
                let width = self.commands.iter().map(|c| c.usage().len()).max();
                println!("Commands:");
                for c in &self.commands {
                    println!(
                        "  {:<width$}  {}",
                        c.usage(),
                        c.help,
                        width = width.unwrap_or(0)
                    );
                }
                Status::Ok
            }
            Builtin::Quit => Status::Quit,
            Builtin::Debug => {
                let on = self.repl.toggle_debug();
                println!("Debug mode {}.", if on { "on" } else { "off" });
                Status::Ok
            }
            Builtin::Type => match self.repl.type_of(arg) {
                Ok(type_) => {
                    println!("{type_}");
                    Status::Ok
                }
                Err(Failed) => Status::Error,
            },
            Builtin::Time => match self.repl.run_timed(arg) {
                Ok(elapsed) => {
                    println!("Time: {}", format_duration(elapsed));
                    Status::Ok
                }
                Err(Failed) => Status::Error,
            },
        }
    }

    /// The names of the repl, the keywords and the commands. What cannot stand
    /// alone -- a keyword that opens something, a command that takes an
    /// argument -- comes with the space that goes after it.
    pub fn completions(&self) -> Vec<String> {
        let mut names = self.repl.completions();
        names.extend(KEYWORDS.iter().map(|s| s.to_string()));
        names.extend(KEYWORDS_OPEN.iter().map(|s| format!("{s} ")));
        names.extend(self.commands.iter().map(|c| match c.arg {
            Arg::None => c.name.to_string(),
            Arg::Expr | Arg::Word(_) => format!("{} ", c.name),
        }));
        names.sort();
        names.dedup();
        names
    }
}

impl Command {
    fn usage(&self) -> String {
        match self.arg {
            Arg::None => self.name.to_string(),
            Arg::Expr => format!("{} <expr>", self.name),
            Arg::Word(word) => format!("{} {word}", self.name),
        }
    }
}

fn status(result: Result<(), Failed>) -> Status {
    match result {
        Ok(()) => Status::Ok,
        Err(Failed) => Status::Error,
    }
}

/// The name of the command at the head of the input and what follows it, or
/// none for Gleam, which never starts with `:`.
fn split(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    if !trimmed.starts_with(':') {
        return None;
    }
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let (name, arg) = trimmed.split_at(end);
    Some((name, arg.trim_start()))
}

/// Where the Gleam of an input begins: at its start, or past the `:type` or
/// `:time` that carries it. Every other command is the whole of its input,
/// the host's included, and has nothing to go on reading for.
///
/// An offset and not the text itself, so that a cursor given in the input's
/// own coordinates can be moved into the Gleam's.
fn gleam_start(input: &str) -> Option<usize> {
    let Some((name, arg)) = split(input) else {
        return Some(0);
    };
    BUILTINS
        .iter()
        .find(|(n, ..)| *n == name)
        .filter(|(_, arg, ..)| matches!(arg, Arg::Expr))
        // `arg` is the tail of the trimmed input, so what is missing from it
        // is everything before where it starts.
        .map(|_| input.len() - input.trim_start().len() + input.trim().len() - arg.len())
}

/// What the prompt does with the input as it stands and where the cursor is in
/// it: `-1` runs the input, and anything else is how far in the line the
/// cursor opens starts, in spaces.
///
/// This is the whole of the question a reader asks on Enter, and both readers
/// ask it here -- the one in the terminal and the one the browser calls
/// through `repl_ready` (see SimpleCode's ENGINE.md).
///
/// An input of one line runs from anywhere in it, the way a prompt always has.
/// One of several runs from the end only: with the cursor back in the text,
/// Enter is opening a line inside a block still being written, and running the
/// block because it happens to be finished is not what the key was pressed
/// for.
///
/// An input with nothing open ends at a blank line, finished or not. That is
/// the only way out of an input that will not close. The user can type an open
/// bracket shut, but `let x =` has no bracket to type, and without this rule
/// the user could only erase the line -- while the error the engine gives for
/// it is the answer they want. With a bracket open the rule would cost more
/// than it gives, taking the blank line between two statements of a function
/// for the end of it.
///
/// `cursor` is a byte offset into `input`, as `repl_complete` already takes
/// one, and one that falls inside a character moves back to the boundary
/// before it -- see [`char_boundary`].
pub fn ready_state(input: &str, cursor: usize) -> i32 {
    let Some(start) = gleam_start(input) else {
        return -1;
    };
    let src = &input[start..];
    // A cursor before the Gleam is not in it at all -- Enter pressed inside
    // `:type` opens the line the expression asks for, the way it would from
    // the end. Clamping to 0 instead answers for a cursor at its start.
    let cursor = char_boundary(src, cursor.checked_sub(start).unwrap_or(src.len()));
    // Nothing but whitespace is left after the cursor: the line it opens is
    // the line after the input, which the input alone already answers.
    let at_end = src[cursor..].trim().is_empty();
    let above = if at_end { src } else { &src[..cursor] };
    let depth = parser::nesting_depth(above);
    if at_end || !src.contains('\n') {
        if !parser::is_incomplete(src) {
            return -1;
        }
        if depth == 0
            && let Some((_, last)) = input.rsplit_once('\n')
            && last.trim().is_empty()
        {
            return -1;
        }
    }
    // The brackets open above the cursor say how far in the line goes, and the
    // code written under it says how far in it has to go: a line shallower
    // than that closes a block the code below still needs open. The deeper of
    // the two, so that neither is crossed.
    (depth * INDENT).max(indent_below(src, cursor)) as i32
}

/// How far in the code under the cursor is written, in spaces: the first line
/// below the cursor's own that says something and can be measured. A blank
/// line and a comment say nothing about where the block is written, and a line
/// indented with anything but spaces cannot be measured in them: all three are
/// passed over.
fn indent_below(src: &str, cursor: usize) -> usize {
    let Some((_, below)) = src[cursor..].split_once('\n') else {
        return 0;
    };
    below
        .lines()
        .find_map(|line| {
            let text = line.trim_start_matches(' ');
            (!text.is_empty() && !text.starts_with("//") && !text.starts_with(char::is_whitespace))
                .then(|| line.len() - text.len())
        })
        .unwrap_or(0)
}

/// What one level of indentation is worth, in spaces.
const INDENT: usize = 2;

/// `cursor` brought inside `text` and onto a char boundary.
///
/// A cursor that is not on one -- which is what a host counting in another
/// unit sends -- moves back to the boundary before it. Slicing at the cursor
/// instead panics, and a panic is the end of the session.
fn char_boundary(text: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(text.len());
    while !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

/// The word around the cursor and where it starts, both in bytes: everything
/// before `cursor`, back to the last char an identifier cannot hold.
pub fn word_at(text: &str, cursor: usize) -> (usize, &str) {
    let before = &text[..char_boundary(text, cursor)];
    let start = before
        .char_indices()
        .rev()
        .find(|(_, c)| is_break_char(*c))
        .map_or(0, |(i, c)| i + c.len_utf8());
    (start, &before[start..])
}

/// Returns `true` if no name of the language has the char in it, so the word
/// being completed ends at it. `:` and `.` are in a name here, as the commands
/// start with one and the qualified names carry the other.
fn is_break_char(c: char) -> bool {
    !c.is_alphanumeric() && c != '_' && c != ':' && c != '.'
}

fn format_duration(elapsed: Duration) -> String {
    if elapsed.as_secs() > 0 {
        format!("{:.2} s", elapsed.as_secs_f64())
    } else if elapsed.as_millis() > 0 {
        format!("{} ms", elapsed.as_millis())
    } else if elapsed.as_micros() > 0 {
        format!("{} µs", elapsed.as_micros())
    } else {
        format!("{} ns", elapsed.as_nanos())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cursor at the end of the input, which is where the tests written
    /// before there was a cursor put it.
    fn ready_state(src: &str) -> i32 {
        super::ready_state(src, src.len())
    }

    /// The `|` in `marked` is where the cursor is.
    fn at_cursor(marked: &str) -> i32 {
        let (above, below) = marked.split_once('|').expect("the cursor is a `|`");
        super::ready_state(&format!("{above}{below}"), above.len())
    }

    #[test]
    fn the_word_at_the_cursor_is_what_comes_before_it() {
        assert_eq!(word_at("let x = lis", 11), (8, "lis"));
        assert_eq!(word_at("lis", 11), (0, "lis"));
        assert_eq!(word_at("1 + f(x", 7), (6, "x"));
        assert_eq!(word_at(":ty", 3), (0, ":ty"));
        assert_eq!(word_at("list.ma", 7), (0, "list.ma"));
        assert_eq!(word_at("let x = lis", 8), (8, ""));
        assert_eq!(word_at("", 0), (0, ""));
    }

    #[test]
    fn the_word_survives_the_chars_that_take_more_than_a_byte() {
        // The break char before it is one of them.
        assert_eq!(word_at("x = \"ol\u{e1}\u{2026}foo", 15), (12, "foo"));
        // The cursor is in the middle of one.
        assert_eq!(word_at("ol\u{e1}_bar", 3), (0, "ol"));
        // The word itself is made of them.
        assert_eq!(word_at("ol\u{e1}_bar", 8), (0, "ol\u{e1}_bar"));
    }

    #[test]
    fn a_command_is_told_apart_from_gleam() {
        assert_eq!(split(" :quit "), Some((":quit", "")));
        assert_eq!(split(":type 1"), Some((":type", "1")));
        assert_eq!(split(":time  f()\n"), Some((":time", "f()")));
        assert_eq!(split(":type"), Some((":type", "")));
        assert_eq!(split(" 1 + 1 "), None);
    }

    #[test]
    fn a_near_miss_of_a_command_is_no_gleam() {
        for input in [":typ x", ":type", ":quit now", ":debug off", ":theme {"] {
            assert_eq!(ready_state(input), -1, "{input:?}");
        }
    }

    #[test]
    fn a_finished_input_is_run() {
        for input in ["1 + 1", "", "let x = 1", "pub fn f() {\n  1\n}", ":quit"] {
            assert_eq!(ready_state(input), -1, "{input:?}");
        }
    }

    #[test]
    fn an_unfinished_input_says_how_far_in_the_next_line_starts() {
        // Nothing is open, so the input is waiting on a value, not on a
        // bracket.
        assert_eq!(ready_state("let x ="), 0);
        assert_eq!(ready_state("1 +"), 0);
        // Every kind of bracket is worth a level.
        assert_eq!(ready_state("case x {"), 2);
        assert_eq!(ready_state("io.println("), 2);
        assert_eq!(ready_state("let x = [1,"), 2);
        assert_eq!(ready_state("let x = #(1,"), 2);
        assert_eq!(ready_state("let x = <<1,"), 2);
        assert_eq!(ready_state("pub fn f() {\n  case x {"), 4);
        // And one that is closed is worth none.
        assert_eq!(ready_state("pub fn f() {\n  f(1)\n  ["), 4);
        // A command is asked about what it carries.
        assert_eq!(ready_state(":type case x {"), 2);
    }

    #[test]
    fn one_line_runs_from_anywhere_in_it_and_several_from_the_end() {
        assert_eq!(at_cursor("let x = |1"), -1);
        // Several lines with the cursor back in them: the key opens a line,
        // finished or not.
        assert_eq!(at_cursor("let x = 1\nlet y = |2"), 0);
    }

    #[test]
    fn the_line_goes_as_deep_as_the_brackets_above_and_the_code_below() {
        // Nothing under the cursor but the closing brace: the depth at the
        // cursor is the whole answer, and it is the canonical one.
        assert_eq!(at_cursor("pub fn f() {\n  let x = 1|\n}"), 2);
        // Code written further in than that is not brought back out by the
        // line the cursor opens: it would leave the line under nothing.
        assert_eq!(
            at_cursor("pub fn f() {\n    let x = 1|\n    let y = 2\n}"),
            4
        );
        // A blank line and a comment say nothing about where the block is.
        assert_eq!(
            at_cursor("pub fn f() {\n    let x = 1|\n\n    // nota\n    let y = 2\n}"),
            4
        );
        // And a cursor past the end is the end.
        assert_eq!(super::ready_state("case x {", 999), 2);
        // A line indented with a tab cannot be measured in spaces, so it is
        // passed over instead of counted as a line written at the margin.
        assert_eq!(
            at_cursor("pub fn f() {\n    let x = 1|\n\tlet y = 2\n    let z = 3\n}"),
            4
        );
    }

    #[test]
    fn a_cursor_inside_a_command_is_a_cursor_at_the_end_of_what_it_carries() {
        // The cursor is in `:type`, not in the Gleam, so there is no line
        // being opened inside the expression: it answers the way it does from
        // the end.
        assert_eq!(at_cursor(":ty|pe case x {"), 2);
        assert_eq!(at_cursor("|:type 1 + 1"), -1);
    }

    #[test]
    fn a_blank_last_line_ends_an_input_with_nothing_open() {
        // The way out of an input that has no bracket left to type.
        assert_eq!(ready_state("let x =\n"), -1);
        assert_eq!(ready_state("let x = \"abc\n"), -1);
        assert_eq!(ready_state(":type 1 +\n"), -1);
        // A line with something on it is not one of these.
        assert_eq!(ready_state("1 +\n  2 +"), 0);
    }

    #[test]
    fn a_blank_line_inside_brackets_is_part_of_the_input() {
        // A function is written with blank lines between its statements, and
        // reading one as the end of the input cuts it in half.
        assert_eq!(ready_state("pub fn f() {\n  let x = 1\n"), 2);
        assert_eq!(ready_state("case x {\n  "), 2);
        assert_eq!(ready_state("let x = [\n  1,\n\n"), 2);
    }

    #[test]
    fn a_duration_is_said_in_its_own_unit() {
        assert_eq!(format_duration(Duration::from_secs(2)), "2.00 s");
        assert_eq!(format_duration(Duration::from_millis(5)), "5 ms");
        assert_eq!(format_duration(Duration::from_micros(7)), "7 \u{b5}s");
        assert_eq!(format_duration(Duration::from_nanos(9)), "9 ns");
    }
}
