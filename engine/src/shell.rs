//! The `:` commands, over a repl that knows none of them.

use std::time::Duration;

use crate::{
    engine::Engine,
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
        "Run one or more expressions and show how long they took",
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
                let usages: Vec<String> = self.commands.iter().map(Command::usage).collect();
                let width = usages.iter().map(String::len).max().unwrap_or(0);
                println!("Commands:");
                for (usage, c) in usages.iter().zip(&self.commands) {
                    println!("  {usage:<width$}  {}", c.help);
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
        // is everything before it.
        .map(|_| input.trim_end().len() - arg.len())
}

/// What the prompt does on Enter with the input as it stands and where the
/// cursor is in it: `-1` runs the input, and anything else is how far in the
/// line the cursor opens starts, in spaces. A command that carries Gleam is
/// answered for the Gleam it carries, and any other command is run as it
/// stands.
///
/// This is the whole of the question a reader asks on Enter, and both readers
/// ask it here -- the one in the terminal and the one the browser calls
/// through `repl_ready` (see SimpleCode's ENGINE.md).
pub fn ready_state(input: &str, cursor: usize) -> i32 {
    let Some(start) = gleam_start(input) else {
        return -1;
    };
    let src = &input[start..];
    // A cursor before the Gleam is not in it at all -- Enter pressed inside
    // `:type` opens the line the expression asks for, the way it would from
    // the end. Clamping to 0 instead answers for a cursor at its start.
    crate::input::ready_state(src, cursor.checked_sub(start).unwrap_or(src.len()))
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
    fn a_command_is_told_apart_from_gleam() {
        assert_eq!(split(" :quit "), Some((":quit", "")));
        assert_eq!(split(":type 1"), Some((":type", "1")));
        assert_eq!(split(":time  f()\n"), Some((":time", "f()")));
        assert_eq!(split(":type"), Some((":type", "")));
        assert_eq!(split(" 1 + 1 "), None);
    }

    #[test]
    fn a_command_that_carries_no_gleam_is_run_as_it_stands() {
        for input in [
            ":quit",
            ":typ x",
            ":type",
            ":quit now",
            ":debug off",
            ":theme {",
        ] {
            assert_eq!(ready_state(input), -1, "{input:?}");
        }
    }

    #[test]
    fn a_command_that_carries_gleam_is_answered_for_what_it_carries() {
        assert_eq!(ready_state(":type case x {"), 2);
        assert_eq!(ready_state(":type 1 +\n"), -1);
        // The cursor is in `:type`, not in the Gleam, so there is no line
        // being opened inside the expression: it answers the way it does from
        // the end.
        assert_eq!(at_cursor(":ty|pe case x {"), 2);
        assert_eq!(at_cursor("|:type 1 + 1"), -1);
    }

    #[test]
    fn a_duration_is_said_in_its_own_unit() {
        assert_eq!(format_duration(Duration::from_secs(2)), "2.00 s");
        assert_eq!(format_duration(Duration::from_millis(5)), "5 ms");
        assert_eq!(format_duration(Duration::from_micros(7)), "7 \u{b5}s");
        assert_eq!(format_duration(Duration::from_nanos(9)), "9 ns");
    }
}
