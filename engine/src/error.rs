use std::io::{IsTerminal as _, Write as _};

use camino::Utf8PathBuf;
use ecow::EcoString;
use gleam_core::build::Origin;
use gleam_core::diagnostic::{Diagnostic, Level};
use indoc::formatdoc;
use termcolor::{BufferWriter, ColorChoice};
use thiserror::Error;

use crate::gleam::relocate_to_user_paths;

#[derive(Debug, Error)]
pub enum SgleamError {
    #[error("invalid smain signature")]
    InvalidSMain {
        module: EcoString,
        signature: EcoString,
    },

    /// A path the user gave from outside the current directory. The path is
    /// what names the module, so a path from outside gives the module no
    /// name.
    #[error("path is not within the current directory")]
    PathNotInCurrentDir {
        current_dir: Utf8PathBuf,
        path: Utf8PathBuf,
    },

    /// A module to run that has neither `main` nor `smain`. The compiler
    /// reports this too, but its hint names the path inside the project.
    #[error("module does not have a main function")]
    NoMainFunction {
        module: EcoString,
        path: Utf8PathBuf,
    },

    /// A directory in the place of a module.
    #[error("path is a directory")]
    PathIsADirectory { path: Utf8PathBuf },

    /// The first path given did not become a module, either because the build
    /// turned it down — saying why as it did — or because nothing compiled it.
    #[error("no module to run")]
    NoModuleToRun { path: Utf8PathBuf },

    /// A failure of the Gleam compiler, shown as the diagnostics it carries,
    /// over the paths the user gave and not the ones the project holds.
    #[error("gleam error")]
    Gleam(gleam_core::Error),

    /// A failure of the QuickJS API itself, such as creating the context or
    /// reading a global. An exception from the code it ran is not one of
    /// these. It arrives as `LauncherScript` or `UserProgramFailed`.
    #[error("quickjs error")]
    QuickJs(rquickjs::Error),

    /// An exception from the script that launches the program, raised before
    /// the program is there to report it. Text, and not the exception itself,
    /// as the JS context behind the message is gone by the time anything
    /// prints it.
    #[error("launcher script error")]
    LauncherScript(String),

    /// A JS runtime error, which the JS side already printed.
    #[error("runtime error")]
    UserProgramFailed,

    /// A check that failed, which said so as it ran.
    #[error("tests failed")]
    TestsFailed,

    /// A run stopped by Ctrl-C. The handler installed with the engine only
    /// raises a flag, which QuickJS turns into an exception at the next
    /// statement of whatever was running.
    #[error("interrupted")]
    Interrupted,

    /// A failure from outside the language: installing the Ctrl-C handler,
    /// starting the repl reader. Shown as it came, as each already names its
    /// own subject.
    #[error(transparent)]
    Other(Box<dyn std::error::Error>),
}

impl From<gleam_core::Error> for SgleamError {
    fn from(value: gleam_core::Error) -> Self {
        SgleamError::Gleam(value)
    }
}

impl From<rquickjs::Error> for SgleamError {
    fn from(value: rquickjs::Error) -> Self {
        SgleamError::QuickJs(value)
    }
}

pub fn show_error(err: &SgleamError) {
    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();

    match err {
        SgleamError::Gleam(err) => write_diagnostics(&mut buffer, &mut err.to_diagnostics()),
        SgleamError::InvalidSMain { module, signature } => error_diagnostic(
            "smain function has an invalid signature",
            format!("`{module}.smain` has the invalid signature `{signature}` and can not be run."),
            // TODO: add an url for more information
            formatdoc! {"
                Use one of the valid signatures for `smain` function:
                  fn() -> a
                  fn(String) -> a
                  fn(List(String)) -> a
                "
            },
        )
        .write(&mut buffer),

        SgleamError::PathNotInCurrentDir { current_dir, path } => error_diagnostic(
            "path is not within the current directory",
            format!("`{path}` is outside of the current directory `{current_dir}`"),
            "Change the current directory or specify another path.".into(),
        )
        .write(&mut buffer),

        SgleamError::NoMainFunction { module, path } => {
            let mut diags = gleam_core::Error::ModuleDoesNotHaveMainFunction {
                module: module.clone(),
                origin: Origin::Src,
            }
            .to_diagnostics();
            for diag in &mut diags {
                diag.hint = Some(format!("Add a public `main` function to `{path}`."));
            }
            write_diagnostics(&mut buffer, &mut diags);
        }

        SgleamError::PathIsADirectory { path } => error_diagnostic(
            "path is a directory",
            format!("`{path}` is a directory, and a module is a file."),
            "Give the path of a `.gleam` file.".into(),
        )
        .write(&mut buffer),

        SgleamError::NoModuleToRun { path } => error_diagnostic(
            "no module to run",
            format!("`{path}` was not compiled into a module."),
            "A module is named after the path of its file, which has to be a `.gleam` file \
             under the current directory."
                .into(),
        )
        .write(&mut buffer),
        SgleamError::Interrupted => {
            writeln!(buffer, "Interrupted.").expect("write to buffer");
        }
        SgleamError::QuickJs(err) => {
            writeln!(buffer, "{err}").expect("write to buffer");
        }
        SgleamError::LauncherScript(message) => {
            writeln!(buffer, "{message}").expect("write to buffer");
        }
        SgleamError::Other(err) => {
            writeln!(buffer, "{err}").expect("write to buffer");
        }
        // The JS runtime already printed it.
        SgleamError::UserProgramFailed | SgleamError::TestsFailed => (),
    };

    flush_buffer(&buffer_writer, &buffer);
}

/// An error of sgleam itself, in the shape the compiler reports its own.
/// Nothing here happened at a place in a file, so there is no location.
fn error_diagnostic(title: &str, text: String, hint: String) -> Diagnostic {
    Diagnostic {
        title: title.into(),
        text,
        hint: Some(hint),
        level: Level::Error,
        location: None,
    }
}

/// Writes the diagnostics to stderr, a blank line apart, each one back on the
/// path the user gave for it.
pub fn show_diagnostics(diags: &mut [Diagnostic]) {
    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();
    write_diagnostics(&mut buffer, diags);
    flush_buffer(&buffer_writer, &buffer);
}

fn write_diagnostics(buffer: &mut termcolor::Buffer, diags: &mut [Diagnostic]) {
    for diagnostic in diags {
        relocate_to_user_paths(diagnostic);
        diagnostic.write(buffer);
        writeln!(buffer).expect("write newline after a diagnostic");
    }
}

fn flush_buffer(buffer_writer: &BufferWriter, buffer: &termcolor::Buffer) {
    buffer_writer.print(buffer).expect("Write to stderr");
}

pub fn stderr_buffer_writer() -> BufferWriter {
    BufferWriter::stderr(if color_stderr() {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    })
}

/// Whether what goes to stderr takes colour, which a diagnostic asks before
/// it is written.
pub fn color_stderr() -> bool {
    use_color(std::io::stderr().is_terminal())
}

/// Whether what goes to stdout takes colour, which the reader asks before it
/// highlights what the student is typing.
pub fn color_stdout() -> bool {
    use_color(std::io::stdout().is_terminal())
}

/// The one rule, for a stream that is or is not a terminal. A terminal wants
/// colour, and a redirected stream keeps the codes out of the file it is
/// being read into.
///
/// `NO_COLOR` with anything in it keeps them out of every stream, as
/// <https://no-color.org> asks, and it wins over `FORCE_COLOR`: the spec
/// gives way to an option written on the command line, and neither of these
/// is one. `FORCE_COLOR` asks for colour where no terminal is watching, which
/// is how a test reads what a student sees.
fn use_color(is_terminal: bool) -> bool {
    !env_is_set("NO_COLOR") && (env_is_set("FORCE_COLOR") || is_terminal)
}

/// Whether the environment holds `name` with something in it. An empty value
/// is not an answer: no-color.org says so of `NO_COLOR`, and `FORCE_COLOR`
/// reads the same way here.
fn env_is_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}
