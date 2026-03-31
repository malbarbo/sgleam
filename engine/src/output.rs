#[cfg(feature = "capture")]
use std::cell::RefCell;

#[cfg(feature = "capture")]
thread_local! {
    static STDOUT_BUF: RefCell<String> = RefCell::new(String::new());
    static STDERR_BUF: RefCell<String> = RefCell::new(String::new());
}

#[cfg(feature = "capture")]
pub fn capture_output<F: FnOnce()>(f: F) -> (String, String) {
    STDOUT_BUF.with(|b| b.borrow_mut().clear());
    STDERR_BUF.with(|b| b.borrow_mut().clear());
    f();
    let out = STDOUT_BUF.with(|b| b.borrow().clone());
    let err = STDERR_BUF.with(|b| b.borrow().clone());
    (out, err)
}

#[cfg(feature = "capture")]
pub fn write_stdout(s: &str) {
    STDOUT_BUF.with(|b| b.borrow_mut().push_str(s));
}

#[cfg(feature = "capture")]
pub fn write_stderr(s: &str) {
    STDERR_BUF.with(|b| b.borrow_mut().push_str(s));
}

#[cfg(not(feature = "capture"))]
pub fn write_stdout(s: &str) {
    std::print!("{s}");
}

#[cfg(not(feature = "capture"))]
pub fn write_stderr(s: &str) {
    std::eprint!("{s}");
}

// When the `capture` feature is enabled, shadow the standard print macros
// so all output goes to thread-local buffers instead of process stdout/stderr.

#[cfg(feature = "capture")]
macro_rules! print {
    ($($arg:tt)*) => { $crate::output::write_stdout(&format!($($arg)*)) };
}

#[cfg(feature = "capture")]
macro_rules! println {
    () => { $crate::output::write_stdout("\n") };
    ($($arg:tt)*) => {{
        $crate::output::write_stdout(&format!($($arg)*));
        $crate::output::write_stdout("\n");
    }};
}

#[cfg(feature = "capture")]
macro_rules! eprint {
    ($($arg:tt)*) => { $crate::output::write_stderr(&format!($($arg)*)) };
}

#[cfg(feature = "capture")]
macro_rules! eprintln {
    () => { $crate::output::write_stderr("\n") };
    ($($arg:tt)*) => {{
        $crate::output::write_stderr(&format!($($arg)*));
        $crate::output::write_stderr("\n");
    }};
}
