//! The page answers, through the imports of the `env` module. What the page has
//! to implement is the whole of `ffi`.

mod ffi {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn check_interrupt() -> bool;
        pub fn sleep(ms: u64);
        pub fn draw_svg(str: *const u8, len: usize);
        pub fn get_key_event(key: *mut u8, len: usize, modifiers: *mut bool) -> usize;

        /// Reads the image and holds it. Returns the length of the data URI,
        /// and 0 when the page cannot read the file.
        pub fn load_bitmap_fetch(path: *const u8, path_len: usize) -> usize;
        pub fn load_bitmap_width() -> f64;
        pub fn load_bitmap_height() -> f64;
        /// Writes the data URI into `buf` and returns how many bytes it wrote.
        pub fn load_bitmap_data(buf: *mut u8, buf_len: usize) -> usize;

        pub fn text_width(
            text: *const u8,
            text_len: usize,
            font_css: *const u8,
            font_css_len: usize,
        ) -> f64;
        pub fn text_height(
            text: *const u8,
            text_len: usize,
            font_css: *const u8,
            font_css_len: usize,
        ) -> f64;
        pub fn text_x_offset(
            text: *const u8,
            text_len: usize,
            font_css: *const u8,
            font_css_len: usize,
        ) -> f64;
        pub fn text_y_offset(
            text: *const u8,
            text_len: usize,
            font_css: *const u8,
            font_css_len: usize,
        ) -> f64;
    }
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

/// The kind of the key event waiting in the page and the name of the key, or
/// nothing at all when the page has no event.
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

pub fn load_bitmap(path: String) -> (f64, f64, String) {
    let data_uri_len = unsafe { ffi::load_bitmap_fetch(path.as_ptr(), path.len()) };
    if data_uri_len == 0 {
        return (0.0, 0.0, String::new());
    }
    let w = unsafe { ffi::load_bitmap_width() };
    let h = unsafe { ffi::load_bitmap_height() };
    let mut buf = vec![0u8; data_uri_len];
    let filled = unsafe { ffi::load_bitmap_data(buf.as_mut_ptr(), buf.len()) };
    buf.truncate(filled);
    let data_uri = String::from_utf8_lossy(&buf).into_owned();
    (w, h, data_uri)
}

pub fn text_width(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_width(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}

pub fn text_height(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_height(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}

pub fn text_x_offset(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_x_offset(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}

pub fn text_y_offset(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_y_offset(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}
