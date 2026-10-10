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

/// `system.sleep` holds the program for `ms` milliseconds. An interruption may
/// end it early.
pub use target::sleep;

/// The bytes of the image file at the path, or the reason the file cannot be
/// read. The browser has no files, so there it always gives the reason.
pub use target::load_bitmap;

/// `system.draw_svg` puts a drawing on the page. Only the browser draws, so a
/// native run has no such function.
#[cfg(target_arch = "wasm32")]
pub use target::draw_svg;

/// Prints a scene in the terminal at the cursor. Returns `true` if the scene
/// went out, `false` if stdout is not a terminal that shows images or the
/// print failed. The browser draws an svg instead, so it has no such
/// function.
#[cfg(not(target_arch = "wasm32"))]
pub use target::print_scene;

/// Shows a frame of a world in a window, or in the terminal when no window
/// opens. The first frame of a run opens the display with its own size.
/// Returns `true` if the frame went out, `false` if neither opened or the
/// frame failed. The browser draws an svg instead, so it has no such
/// function.
#[cfg(not(target_arch = "wasm32"))]
pub use target::show_scene;

/// Closes the display of the frames, so the run that comes next opens its
/// own.
#[cfg(not(target_arch = "wasm32"))]
pub use target::close_frames;

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
