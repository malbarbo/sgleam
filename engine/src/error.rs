use std::io::{IsTerminal as _, Write as _};

use camino::Utf8PathBuf;
use ecow::EcoString;
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
        SgleamError::InvalidSMain { module, signature } => Diagnostic {
            title: "smain function has an invalid signature".into(),
            text: format!(
                "`{module}.smain` has the invalid signature `{signature}` and can not be run."
            ),
            // TODO: add an url for more information
            hint: Some(formatdoc! {"
                Use one of the valid signatures for `smain` function:
                  fn() -> a
                  fn(String) -> a
                  fn(List(String)) -> a
                "
            }),
            level: Level::Error,
            location: None,
        }
        .write(&mut buffer),

        SgleamError::PathNotInCurrentDir { current_dir, path } => Diagnostic {
            title: "path is not within the current directory".into(),
            text: format!("`{path}` is outside of the current directory `{current_dir}`"),
            hint: Some("Change the current directory or specify another path.".into()),
            level: Level::Error,
            location: None,
        }
        .write(&mut buffer),

        SgleamError::PathIsADirectory { path } => Diagnostic {
            title: "path is a directory".into(),
            text: format!("`{path}` is a directory, and a module is a file."),
            hint: Some("Give the path of a `.gleam` file.".into()),
            level: Level::Error,
            location: None,
        }
        .write(&mut buffer),

        SgleamError::NoModuleToRun { path } => Diagnostic {
            title: "no module to run".into(),
            text: format!("`{path}` was not compiled into a module."),
            hint: Some(
                "A module is named after the path of its file, which has to be a \
                 `.gleam` file under the current directory."
                    .into(),
            ),
            level: Level::Error,
            location: None,
        }
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

pub fn flush_buffer(buffer_writer: &BufferWriter, buffer: &termcolor::Buffer) {
    buffer_writer.print(buffer).expect("Write to stderr");
}

pub fn stderr_buffer_writer() -> BufferWriter {
    BufferWriter::stderr(color_choice())
}

fn color_choice() -> ColorChoice {
    if colour_forced() || std::io::stderr().is_terminal() {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    }
}

fn colour_forced() -> bool {
    std::env::var("FORCE_COLOR").is_ok_and(|v| !v.is_empty())
}
