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

    #[error("path is not within the current directory")]
    PathNotInCurrentDir {
        current_dir: Utf8PathBuf,
        path: Utf8PathBuf,
    },

    #[error("no module to run")]
    NoModuleToRun { path: Utf8PathBuf },

    #[error("gleam error")]
    Gleam(gleam_core::Error),

    #[error("quickjs error")]
    QuickJs(rquickjs::Error),

    /// An exception from the script that launches the program, raised before
    /// the program is there to report it. Kept as text: the JS context the
    /// message comes from is gone by the time it is shown.
    #[error("launcher script error")]
    LauncherScript(String),

    /// A JS runtime error that was already displayed by the JS side.
    #[error("runtime error")]
    UserProgramFailed,

    /// A check that failed, which said so as it ran.
    #[error("tests failed")]
    TestsFailed,

    #[error("interrupted")]
    Interrupted,

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
        SgleamError::Gleam(err) => {
            for mut diagnostic in err.to_diagnostics() {
                relocate_to_user_paths(&mut diagnostic);
                diagnostic.write(&mut buffer);
                writeln!(buffer).expect("write newline after a diagnostic");
            }
        }
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
        // Already displayed by the JS runtime.
        SgleamError::UserProgramFailed | SgleamError::TestsFailed => return,
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
    };

    flush_buffer(&buffer_writer, &buffer);
}

pub fn flush_buffer(buffer_writer: &BufferWriter, buffer: &termcolor::Buffer) {
    buffer_writer.print(buffer).expect("Write to stderr");
}

pub fn stderr_buffer_writer() -> BufferWriter {
    // Don't add color codes to the output if standard error isn't connected to a terminal
    BufferWriter::stderr(color_choice())
}

fn colour_forced() -> bool {
    std::env::var("FORCE_COLOR").is_ok_and(|v| !v.is_empty())
}

fn color_choice() -> ColorChoice {
    if colour_forced() || std::io::stderr().is_terminal() {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    }
}
