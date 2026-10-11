//! The page answers, through the imports of the `env` module. What the page has
//! to implement is the whole of `ffi`.

use crate::error::SgleamError;

mod ffi {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn check_interrupt() -> bool;
        pub fn sleep(ms: u64);
        pub fn draw_svg(str: *const u8, len: usize);

        /// Waits at most `ms` milliseconds for an event, and returns its kind
        /// as an index into keypress, keydown, keyup and close. Any other
        /// value says that no event came. For a key, writes its name into
        /// `key`, at most `len` bytes of it, and one bool per modifier into
        /// `modifiers`, which holds five, in the order alt, ctrl, shift, meta
        /// and repeat.
        pub fn wait_event(ms: u64, key: *mut u8, len: usize, modifiers: *mut bool) -> usize;
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

pub fn sleep(ms: u64) {
    unsafe { ffi::sleep(ms) };
}

pub fn draw_svg(str: String) {
    unsafe { ffi::draw_svg(str.as_ptr(), str.len()) }
}

/// The kind first, then the name of the key, then one name per modifier that
/// is on. A close is the kind alone.
pub fn wait_event(ms: u64) -> Vec<String> {
    let mut key = [0u8; 32];
    let mut modifiers = [false; 5];
    let result =
        unsafe { ffi::wait_event(ms, key.as_mut_ptr(), key.len(), modifiers.as_mut_ptr()) };
    match ["keypress", "keydown", "keyup", "close"].get(result) {
        Some(&"close") => vec!["close".into()],
        Some(kind) => key_event(kind, &key, &modifiers),
        None => vec![],
    }
}

fn key_event(kind: &str, key: &[u8], modifiers: &[bool]) -> Vec<String> {
    let mut ret = vec![
        kind.into(),
        String::from_utf8_lossy(key)
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
}

pub fn load_bitmap(_path: &str) -> Result<Vec<u8>, String> {
    Err("the web version has no files".into())
}
