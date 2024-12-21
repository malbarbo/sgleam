use std::io::{IsTerminal as _, Write as _};

use camino::Utf8PathBuf;
use ecow::EcoString;
use gleam_core::diagnostic::{Diagnostic, Level};
use indoc::formatdoc;
use termcolor::{BufferWriter, ColorChoice};
use thiserror::Error;

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

    #[error("gleam error")]
    Gleam(gleam_core::Error),

    #[error("quickjs error")]
    QuickJs(rquickjs::Error),

    #[error("rustline error")]
    Rustline(rustyline::error::ReadlineError),
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

impl From<rustyline::error::ReadlineError> for SgleamError {
    fn from(value: rustyline::error::ReadlineError) -> Self {
        SgleamError::Rustline(value)
    }
}

pub fn show_error(err: &SgleamError) {
    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();

    match err {
        SgleamError::Gleam(err) => {
            err.pretty(&mut buffer);
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
        // TODO: improve error
        _ => {
            writeln!(buffer, "{:?}", err).expect("write to buffer");
        }
    };

    buffer_writer
        .print(&buffer)
        .expect("Write warning to stderr");
}

pub fn stderr_buffer_writer() -> BufferWriter {
    // Don't add color codes to the output if standard error isn't connected to a terminal
    BufferWriter::stderr(color_choice())
}

fn colour_forced() -> bool {
    if let Ok(force) = std::env::var("FORCE_COLOR") {
        !force.is_empty()
    } else {
        false
    }
}

fn color_choice() -> ColorChoice {
    if colour_forced() {
        ColorChoice::Always
    } else if std::io::stderr().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    }
}
