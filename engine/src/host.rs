//! What a program asks of the world around it. Each target answers on its own:
//! natively the operating system does, and on wasm32 the page that loaded the
//! module, through the imports of the `env` module.

use std::sync::atomic::{AtomicBool, Ordering};

static STOP: AtomicBool = AtomicBool::new(false);

/// One flag for the whole process: whatever runs stops at the next check the
/// engine makes. Natively that check reads the flag; in the browser the page
/// answers instead, and nothing reads it.
pub fn interrupt() {
    STOP.store(true, Ordering::Relaxed);
}

/// Milliseconds since the epoch, which is what `system.now_ms` gives a program
/// and how `world` times its ticks. One implementation serves both targets:
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

/// The kind of the key event the page has for us and the key it names, or
/// nothing at all when no key was pressed.
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
