//! What a program asks of the world around it. The operating system answers
//! natively, and in the browser the page answers, through the imports of the
//! `env` module.

use std::sync::atomic::{AtomicBool, Ordering};

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

#[cfg(not(target_arch = "wasm32"))]
pub fn check_interrupt() -> bool {
    STOP.swap(false, Ordering::Relaxed)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

#[cfg(target_arch = "wasm32")]
mod ffi {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn check_interrupt() -> bool;
        pub fn sleep(ms: u64);
        pub fn draw_svg(str: *const u8, len: usize);
        pub fn get_key_event(key: *mut u8, len: usize, modifiers: *mut bool) -> usize;
    }
}

#[cfg(target_arch = "wasm32")]
pub fn check_interrupt() -> bool {
    unsafe { ffi::check_interrupt() }
}

#[cfg(target_arch = "wasm32")]
pub fn sleep(ms: u64) {
    unsafe { ffi::sleep(ms) };
}

#[cfg(target_arch = "wasm32")]
pub fn draw_svg(str: String) {
    unsafe { ffi::draw_svg(str.as_ptr(), str.len()) }
}

/// The kind of the key event waiting in the page and the name of the key, or
/// nothing at all when the page has no event.
#[cfg(target_arch = "wasm32")]
pub fn get_key_event() -> Vec<String> {
    let mut key = [0u8; 32];
    let mut modifiers = [false; 5];
    let result = unsafe { ffi::get_key_event(key.as_mut_ptr(), key.len(), modifiers.as_mut_ptr()) };
    if let Some(type_) = ["keypress", "keydown", "keyup"].get(result) {
        let mut ret = vec![
            (*type_).into(),
            String::from_utf8_lossy(&key)
                .trim_matches(char::from(0))
                .to_string(),
        ];
        for (on, key) in modifiers
            .iter()
            .zip(&["alt", "ctrl", "shift", "meta", "repeat"])
        {
            if *on {
                ret.push((*key).into())
            }
        }
        ret
    } else {
        vec![]
    }
}
