//! The operating system answers. Ctrl-C stops a program, the file system holds
//! an image and resvg measures a piece of text.

use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};

use crate::error::SgleamError;

/// One flag serves the whole process, so an interruption reaches every engine
/// at once.
static STOP: AtomicBool = AtomicBool::new(false);

fn interrupt() {
    STOP.store(true, Ordering::Relaxed);
}

/// Puts the Ctrl-C handler in place. A failure stays as well, so a second
/// engine does not run quietly with no way to stop it.
pub fn init() -> Result<(), SgleamError> {
    static CTRLC: OnceLock<Result<(), String>> = OnceLock::new();
    // What ctrlc says already names its subject, so the message goes on as it
    // came.
    CTRLC
        .get_or_init(|| ctrlc::set_handler(interrupt).map_err(|err| err.to_string()))
        .clone()
        .map_err(|err| SgleamError::Other(err.into()))
}

pub fn check_interrupt() -> bool {
    STOP.swap(false, Ordering::Relaxed)
}

pub fn sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

// The file system holds the image, and its header says what it is.
#[path = "native/bitmap.rs"]
mod bitmap;
pub use bitmap::load_bitmap;

// resvg measures the text, or a guess from the font does.
#[path = "native/text.rs"]
mod text;
pub use text::text_metrics;
