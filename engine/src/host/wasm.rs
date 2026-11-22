//! The page answers, through the imports of the `env` module and the session
//! of sinteract on fd 3 and fd 4. What the page has to implement is the whole
//! of `ffi`, and the WASI calls of ENGINE.md in simplecode.

use std::num::NonZeroU32;
use std::time::Duration;

use sinteract::display::StageMode;

use crate::error::SgleamError;

mod ffi {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn check_interrupt() -> bool;
        pub fn draw_svg(str: *const u8, len: usize);
        pub fn draw_view(str: *const u8, len: usize) -> u32;
        pub fn next_event(buf: *mut u8, len: usize) -> usize;
        pub fn wait_event(timeout_ms: i32) -> i32;
        pub fn http_request(str: *const u8, len: usize) -> usize;
        pub fn http_response(buf: *mut u8, len: usize) -> usize;
    }
}

/// The page is there before the first program runs, so there is nothing to
/// make ready.
pub fn init() -> Result<(), SgleamError> {
    Ok(())
}

pub fn check_interrupt() -> bool {
    unsafe { ffi::check_interrupt() }
}

/// The page clears the request as a run starts, before it formats the code of
/// a program, so a Stop pressed during the format still stops the program.
pub fn clear_interrupt() {}

/// The page ends the wait of `poll_oneoff` when the user presses Stop.
pub fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

pub fn draw_svg(str: String) {
    unsafe { ffi::draw_svg(str.as_ptr(), str.len()) }
}

pub fn draw_view(html: String) -> u32 {
    unsafe { ffi::draw_view(html.as_ptr(), html.len()) }
}

pub fn next_view_event() -> String {
    let mut buf = vec![0u8; 512];
    loop {
        let n = unsafe { ffi::next_event(buf.as_mut_ptr(), buf.len()) };
        if n <= buf.len() {
            buf.truncate(n);
            return String::from_utf8_lossy(&buf).into_owned();
        }
        // The event stays in the queue until a buffer big enough takes it.
        buf.resize(n, 0);
    }
}

pub fn wait_view_event(timeout_ms: i32) {
    unsafe { ffi::wait_event(timeout_ms) };
}

pub fn http_request(request: String) -> String {
    let n = unsafe { ffi::http_request(request.as_ptr(), request.len()) };
    let mut buf = vec![0u8; n];
    let n = unsafe { ffi::http_response(buf.as_mut_ptr(), buf.len()) };
    buf.truncate(n);
    String::from_utf8_lossy(&buf).into_owned()
}

/// The session decides the players of a world on the page.
pub(super) fn players(_options: HostOptions) -> Option<NonZeroU32> {
    None
}

/// The page has no choice to make about its displays.
#[derive(Clone, Copy, Debug, Default)]
pub struct HostOptions;

/// The page shows each world in a session, the only display it has.
pub(super) fn stage_mode(_options: HostOptions) -> StageMode {
    StageMode::Session
}
