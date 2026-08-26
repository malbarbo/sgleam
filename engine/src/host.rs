//! What a program asks of the world around it. The page answers in `wasm` and
//! the operating system in `native`, and what is here holds for both.

use std::sync::atomic::{AtomicBool, Ordering};

pub use target::{check_interrupt, sleep};
#[cfg(target_arch = "wasm32")]
pub use target::{draw_svg, get_key_event};

/// `system.load_bitmap` reads an image file and gives a program the width and
/// the height of the image, and its bytes as a data URI, which a drawing puts
/// in an `<image>` element. Zeros and an empty string say that the file is
/// missing, or that nothing here reads a file of that kind.
pub use target::load_bitmap;

/// What `system.text_width` and its neighbours give a program. The width and
/// the height are those of the box around the text. The offsets go from the
/// middle of that box to the origin of an svg `<text>` element, which sits at
/// the start of the baseline.
pub use target::{text_height, text_width, text_x_offset, text_y_offset};

#[cfg(target_arch = "wasm32")]
#[path = "host/wasm.rs"]
mod target;
#[cfg(not(target_arch = "wasm32"))]
#[path = "host/native.rs"]
mod target;

static STOP: AtomicBool = AtomicBool::new(false);

/// Stops the running program at its next check for an interruption. One flag
/// serves the whole process, so an interruption reaches every engine at once.
/// Natively the engine reads the flag. In the browser the page answers the
/// check itself and nothing reads the flag.
pub fn interrupt() {
    STOP.store(true, Ordering::Relaxed);
}

/// Milliseconds since the epoch, which is what `system.now_ms` gives a program
/// and how `world` times its ticks. One implementation serves both targets.
/// `SystemTime` reads the WASI clock on wasm32-wasip1 and the system clock
/// natively.
pub fn now_ms() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
