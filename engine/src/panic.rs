// This file is from gleam project
// compiler-cli/src/panic.rs, except the message, which names sgleam, the failed
// print, which gleam has no case for, and the errors, which gleam unwraps.

use std::io::Write as _;
use std::panic::PanicHookInfo;

use termcolor::{Color, ColorSpec, WriteColor as _};

use crate::error::stderr_buffer_writer;

pub fn add_handler() {
    std::panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
        if is_a_failed_print(&panic_message(info)) {
            // The report would go to the stream that just failed, and a pipe
            // the reader closed is not a bug of ours.
            std::process::exit(1);
        }
        if print_compiler_bug_message(info).is_err() {
            std::process::exit(1);
        }
    }));
}

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
    write!(buffer, ": Fatal bug!\n\n")?;
    buffer.set_color(&ColorSpec::new())?;
    writeln!(
        buffer,
        "This is a bug in sgleam, sorry!

Please report this crash to https://github.com/malbarbo/sgleam/issues/new
and include this error message with your report.

Panic: {location}{message}
Version: {version}
Operating system: {os}

If you can also share your code and say what file you were editing or any
steps to reproduce the crash that would be a great help.

You may also want to try again with the `GLEAM_LOG=trace` environment
variable set.
",
        location = location,
        message = message,
        version = crate::version(),
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
        assert!(is_a_failed_print(
            "failed printing to stderr: Rohrleitung unterbrochen (os error 32)"
        ));
        assert!(!is_a_failed_print("index out of bounds: the len is 0"));
    }
}
