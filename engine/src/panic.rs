// This file is from gleam project.

use std::io::Write as _;
use std::panic::PanicHookInfo;

use termcolor::{Color, ColorSpec, WriteColor as _};

use crate::error::stderr_buffer_writer;

pub fn add_handler() {
    std::panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
        // Nowhere left to print to is not a bug in the compiler, and the report
        // would go where the output already could not. On unix the process
        // never gets here: the write raises SIGPIPE, which the cli puts back to
        // killing it. Windows has no such signal, so the failure is handed to
        // the print, which panics on it.
        if is_a_failed_print(&panic_message(info)) {
            // Without a word, as a reader that went away almost always is what
            // this is; and not with a success, as a disk that filled did not
            // get written either.
            std::process::exit(1);
        }
        if print_compiler_bug_message(info).is_err() {
            println!("Failed to print compiler bug message.");
        }
    }));
}

/// What `println!` and its family panic with when the write under them fails.
/// The error itself is gone by then — std formats it into the message, whose
/// tail the system words and translates — so the prefix is all to go by.
fn is_a_failed_print(message: &str) -> bool {
    message.starts_with("failed printing to ")
}

fn panic_message(info: &PanicHookInfo<'_>) -> String {
    match (
        info.payload().downcast_ref::<&str>(),
        info.payload().downcast_ref::<String>(),
    ) {
        (Some(s), _) => (*s).to_string(),
        (_, Some(s)) => s.to_string(),
        (None, None) => "unknown error".into(),
    }
}

fn print_compiler_bug_message(info: &PanicHookInfo<'_>) -> std::io::Result<()> {
    let message = panic_message(info);
    let location = match info.location() {
        None => "".into(),
        Some(location) => format!("{}:{}\n\t", location.file(), location.line()),
    };

    let buffer_writer = stderr_buffer_writer();
    let mut buffer = buffer_writer.buffer();
    buffer.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Red)))?;
    write!(buffer, "error")?;
    buffer.set_color(ColorSpec::new().set_bold(true))?;
    write!(buffer, ": Fatal compiler bug!\n\n")?;
    buffer.set_color(&ColorSpec::new())?;
    writeln!(
        buffer,
        "This is a bug in the Gleam compiler, sorry!

Please report this crash to https://github.com/gleam-lang/gleam/issues/new
and include this error message with your report.

Panic: {location}{message}
Gleam version: {version}
Operating system: {os}

If you can also share your code and say what file you were editing or any
steps to reproduce the crash that would be a great help.

You may also want to try again with the `GLEAM_LOG=trace` environment
variable set.
",
        location = location,
        message = message,
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
    )?;
    buffer_writer.print(&buffer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_a_failed_print;

    /// The messages are std's, from `print_to` in `std::io::stdio`.
    #[test]
    fn a_write_that_failed_is_not_a_bug() {
        assert!(is_a_failed_print(
            "failed printing to stdout: Broken pipe (os error 32)"
        ));
        // Whatever the system called it, and wherever it was going.
        assert!(is_a_failed_print(
            "failed printing to stderr: Rohrleitung unterbrochen (os error 32)"
        ));
        assert!(!is_a_failed_print("index out of bounds: the len is 0"));
    }
}
