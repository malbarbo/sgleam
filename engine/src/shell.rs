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

/// What became of an input. The numbers are what `repl_run` answers with.
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
    /// An optional word, shown in the help as given.
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

/// The keywords something else always follows, and so the ones the completion
/// hands back with the space already typed, as it does for `:type`.
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
        let Some(index) = self.commands.iter().position(|c| c.name == name) else {
            println!("Unknown command {name}. Type :help to see the commands.");
            return Status::Error;
        };
        let command = &mut self.commands[index];
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

/// The name of the command the input starts with and what follows it, or none
/// for Gleam, which never starts with `:`.
fn split(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    if !trimmed.starts_with(':') {
        return None;
    }
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let (name, arg) = trimmed.split_at(end);
    Some((name, arg.trim_start()))
}

/// The Gleam of an input: all of it, or what a `:type` or `:time` carries.
/// Every other command is the whole of its input, the host's included.
fn gleam(input: &str) -> Option<&str> {
    let Some((name, arg)) = split(input) else {
        return Some(input);
    };
    BUILTINS
        .iter()
        .find(|(n, ..)| *n == name)
        .filter(|(_, arg, ..)| matches!(arg, Arg::Expr))
        .map(|_| arg)
}

/// What the prompt does with the input as it stands: `-1` runs it, and
/// anything else is how far in the next line starts, in spaces.
///
/// This is the whole of the question a reader asks on Enter, and both readers
/// ask it here -- the one in the terminal and the one the browser calls
/// through `repl_ready` (see SimpleCode's ENGINE.md).
///
/// An input with nothing open ends at a blank line, finished or not. That is
/// the only way out of one that will not close: an open bracket can be typed
/// shut, but `let x =` has none to type, and without this the line can only be
/// erased -- while the error the engine gives for it is the answer the user is
/// after. With a bracket open the rule would cost more than it gives, taking
/// the blank line between two statements of a function for the end of it.
pub fn ready_state(input: &str) -> i32 {
    let Some(gleam) = gleam(input) else {
        return -1;
    };
    if !parser::is_incomplete(gleam) {
        return -1;
    }
    let depth = parser::nesting_depth(gleam);
    if depth == 0
        && let Some((_, last)) = input.rsplit_once('\n')
        && last.trim().is_empty()
    {
        return -1;
    }
    (depth * INDENT) as i32
}

/// What one level of indentation is worth, in spaces.
const INDENT: usize = 2;

/// The word the cursor is in and where it starts, both in bytes: what comes
/// before `cursor`, back to the last char an identifier cannot hold.
///
/// A cursor that is not on a char boundary -- which is what a host counting
/// in another unit sends -- is taken back to the boundary before it. Slicing
/// there instead panics, and a panic is the end of the session.
pub fn word_at(text: &str, cursor: usize) -> (usize, &str) {
    let mut end = cursor.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let start = before
        .char_indices()
        .rev()
        .find(|(_, c)| is_break_char(*c))
        .map_or(0, |(i, c)| i + c.len_utf8());
    (start, &before[start..])
}

/// A char no name of the language has in it, and so one the word being
/// completed ends at. `:` and `.` are in a name here: the commands start with
/// one and the qualified names carry the other.
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
        // Nothing is open: the input is waiting on a value, not on a bracket.
        assert_eq!(ready_state("let x ="), 0);
        assert_eq!(ready_state("1 +"), 0);
        // Every kind of bracket is worth a level.
        assert_eq!(ready_state("case x {"), 2);
        assert_eq!(ready_state("io.println("), 2);
        assert_eq!(ready_state("let x = [1,"), 2);
        assert_eq!(ready_state("let x = #(1,"), 2);
        assert_eq!(ready_state("pub fn f() {\n  case x {"), 4);
        // And one that is closed is worth none.
        assert_eq!(ready_state("pub fn f() {\n  f(1)\n  ["), 4);
        // A command is asked about what it carries.
        assert_eq!(ready_state(":type case x {"), 2);
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
