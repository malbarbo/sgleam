//! What a program asks of the world around it. The page answers in `wasm` and
//! the operating system in `native`.
//!
//! Each target provides every function re-exported below, under the name and
//! with the meaning written here. A `cfg` on a re-export says that only one
//! target has the function, and whatever asks for it says the same `cfg`.

/// Makes the world ready to answer. An engine calls it as it is built. The
/// first call does the work and a later one gives back its result.
pub use target::init;

/// Returns `true` if an interruption is waiting, `false` otherwise. Reading the
/// request clears it, so the run after an interrupted one does not stop at its
/// first check.
pub use target::check_interrupt;

/// `system.sleep` holds the program for `ms` milliseconds.
pub use target::sleep;

/// `system.load_bitmap` reads an image file and gives the width and the height
/// of the image, and its bytes as a data URI, which a drawing puts in an
/// `<image>` element. Zeros and an empty string say that the file is missing,
/// or that nothing here reads a file of that kind.
pub use target::load_bitmap;

/// What `system.text_metrics` gives a program, in order. The width and the
/// height are those of the box around the text. The offsets go from the middle
/// of that box to the origin of an svg `<text>` element, which sits at the
/// start of the baseline.
pub use target::text_metrics;

/// `system.draw_svg` puts a drawing on the page. Only the browser draws, so a
/// native run has no such function.
#[cfg(target_arch = "wasm32")]
pub use target::draw_svg;

/// `system.get_key_event` gives the kind of the event waiting in the page, the
/// name of the key and the modifiers that are on, or nothing when no event
/// waits. Only the browser reads keys, so a native run has no such function.
#[cfg(target_arch = "wasm32")]
pub use target::get_key_event;

#[cfg(target_arch = "wasm32")]
#[path = "host/wasm.rs"]
mod target;
#[cfg(not(target_arch = "wasm32"))]
#[path = "host/native.rs"]
mod target;

/// `system.now_ms` gives milliseconds since the epoch, and `world` times its
/// ticks by it.
pub fn now_ms() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
