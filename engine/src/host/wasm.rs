//! The page answers, through the imports of the `env` module. What the page has
//! to implement is the whole of `ffi`.

use crate::error::SgleamError;

mod ffi {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn check_interrupt() -> bool;
        pub fn sleep(ms: u64);
        pub fn draw_svg(str: *const u8, len: usize);

        /// Returns the kind of the event waiting in the page, as an index
        /// into keypress, keydown and keyup. Any other value says that no
        /// event waits. Writes the name of the key into `key`, at most `len`
        /// bytes of it, and one bool per modifier into `modifiers`, which
        /// holds five, in the order alt, ctrl, shift, meta and repeat.
        pub fn get_key_event(key: *mut u8, len: usize, modifiers: *mut bool) -> usize;

        /// Reads the image and holds it. Returns the length of the data URI,
        /// and 0 when the page cannot read the file.
        pub fn load_bitmap_fetch(path: *const u8, path_len: usize) -> usize;
        /// Writes the data URI into `buf` and returns how many bytes it wrote.
        pub fn load_bitmap_data(buf: *mut u8, buf_len: usize) -> usize;
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
/// is on.
pub fn get_key_event() -> Vec<String> {
    let mut key = [0u8; 32];
    let mut modifiers = [false; 5];
    let result = unsafe { ffi::get_key_event(key.as_mut_ptr(), key.len(), modifiers.as_mut_ptr()) };
    let Some(type_) = ["keypress", "keydown", "keyup"].get(result) else {
        return vec![];
    };
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
}

/// The page gives the file as a data URI.
pub fn load_bitmap(path: &str) -> Result<Vec<u8>, String> {
    let data_uri_len = unsafe { ffi::load_bitmap_fetch(path.as_ptr(), path.len()) };
    if data_uri_len == 0 {
        return Err("the page cannot fetch the file".into());
    }
    let mut buf = vec![0u8; data_uri_len];
    let filled = unsafe { ffi::load_bitmap_data(buf.as_mut_ptr(), buf.len()) };
    buf.truncate(filled);
    std::str::from_utf8(&buf)
        .ok()
        .and_then(sinteract::asset::decode_data_uri)
        .ok_or_else(|| "the page gives no data URI".into())
}
