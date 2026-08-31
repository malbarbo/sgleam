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

pub fn text_metrics(text: String, font_css: String) -> (f64, f64, f64, f64) {
    measure(&text, &font_css)
}

/// The size in pixels and which part of the css says it. The part that ends in
/// `px` gives the size, the parts before it say the style and the weight, and
/// the parts after it name the family. Without such a part the size is 14.
fn font_size(parts: &[&str]) -> (f64, Option<usize>) {
    for (i, part) in parts.iter().enumerate() {
        if let Some(s) = part.strip_suffix("px")
            && let Ok(size) = s.parse::<f64>()
        {
            return (size, Some(i));
        }
    }
    (14.0, None)
}

/// A guess from the size of the font alone. The width of a character is a fixed
/// part of the size and the height of a line is the size. The offsets place the
/// text where the measured ones place it, half the box to the left and the
/// baseline four fifths of the size below the top.
fn heuristic(text: &str, font_css: &str) -> (f64, f64, f64, f64) {
    let (size, _) = font_size(&font_css.split_whitespace().collect::<Vec<_>>());
    let width = text.chars().count() as f64 * size * 0.6;
    let ascent = size * 0.8;
    (width, size, -width / 2.0, ascent - size / 2.0)
}

// resvg measures the text, and the guess stands in without it.
#[cfg(feature = "resvg")]
#[path = "native/layout.rs"]
mod layout;
#[cfg(feature = "resvg")]
use layout::measure;

#[cfg(not(feature = "resvg"))]
fn measure(text: &str, font_css: &str) -> (f64, f64, f64, f64) {
    heuristic(text, font_css)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_css_with_no_size_in_pixels_has_the_default() {
        assert_eq!(font_size(&["bold", "16px", "monospace"]), (16.0, Some(1)));
        assert_eq!(font_size(&["bold", "large"]), (14.0, None));
    }

    #[test]
    fn the_guess_grows_with_the_text_and_the_size() {
        let (w, h, x, y) = heuristic("abc", "10px sans-serif");
        assert_eq!((h, x, y), (10.0, -w / 2.0, 3.0));
        assert!(w > heuristic("ab", "10px sans-serif").0);
        assert!(heuristic("abc", "20px sans-serif").0 > w);
    }
}
