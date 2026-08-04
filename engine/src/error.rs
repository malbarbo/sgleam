use std::io::{IsTerminal as _, Write as _};

use camino::Utf8PathBuf;
use ecow::EcoString;
use gleam_core::diagnostic::{Diagnostic, Label, Level, Location};
use indoc::formatdoc;
use termcolor::{BufferWriter, ColorChoice};
use thiserror::Error;

use crate::substitution::SubstitutionError;

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

    /// A JS runtime error that was already displayed by the JS side.
    #[error("runtime error")]
    UserProgramRuntimeError,

    #[error("interrupted")]
    Interrupted,

    #[error("substitution error")]
    Substitution(SubstitutionError),

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

impl From<std::io::Error> for SgleamError {
    fn from(value: std::io::Error) -> Self {
        SgleamError::Other(Box::new(value))
    }
}

impl From<SubstitutionError> for SgleamError {
    fn from(value: SubstitutionError) -> Self {
        SgleamError::Substitution(value)
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
        // Already displayed by the JS runtime.
        SgleamError::UserProgramRuntimeError => return,
        SgleamError::Interrupted => {
            writeln!(buffer, "Interrupted.").expect("write to buffer");
        }
        SgleamError::QuickJs(err) => {
            writeln!(buffer, "{err}").expect("write to buffer");
        }
        SgleamError::Other(err) => {
            writeln!(buffer, "{err}").expect("write to buffer");
        }
        SgleamError::Substitution(err) => match err {
            SubstitutionError::UnsupportedFeature {
                kind,
                location,
                src,
                path,
            } => {
                Diagnostic {
                    title: "Unsupported feature in stepper".into(),
                    text: format!("The stepper does not support {kind} yet."),
                    hint: Some(format!(
                        "The stepper is designed for learning basic logic. Try simplifying this {kind}."
                    )),
                    level: Level::Error,
                    location: Some(Location {
                        src: src.clone(),
                        path: path.clone(),
                        label: Label {
                            span: *location,
                            text: Some("this feature is not supported".into()),
                        },
                        extra_labels: vec![],
                    }),
                }
                .write(&mut buffer);
            }
            SubstitutionError::StepLimitExceeded(limit) => {
                Diagnostic {
                    title: "Evaluation timeout".into(),
                    text: format!("The substitution exceeded the limit of {limit} steps."),
                    hint: Some(
                        "This usually happens in infinite recursions. Check your logic.".into(),
                    ),
                    level: Level::Error,
                    location: None,
                }
                .write(&mut buffer);
            }
            SubstitutionError::FormattingError => {
                Diagnostic {
                    title: "Internal error".into(),
                    text: "The stepper failed to format a substitution step.".into(),
                    hint: None,
                    level: Level::Error,
                    location: None,
                }
                .write(&mut buffer);
            }
            SubstitutionError::Format(e) => e.pretty(&mut buffer),
        },
    };

    flush_buffer(&buffer_writer, &buffer);
}

pub fn flush_buffer(_buffer_writer: &BufferWriter, buffer: &termcolor::Buffer) {
    #[cfg(feature = "capture")]
    eprint!("{}", String::from_utf8_lossy(buffer.as_slice()));
    #[cfg(not(feature = "capture"))]
    _buffer_writer.print(buffer).expect("Write to stderr");
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
