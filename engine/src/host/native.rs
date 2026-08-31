//! The operating system answers. Ctrl-C stops a program, the file system holds
//! an image and resvg measures a piece of text.

use base64::Engine as _;
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

pub fn load_bitmap(path: String) -> (f64, f64, String) {
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Error reading {path}: {err}");
            return (0.0, 0.0, String::new());
        }
    };
    match image_header(&data) {
        Some((mime, width, height)) if width > 0 && height > 0 => {
            let base64 = base64::engine::general_purpose::STANDARD.encode(&data);
            (
                width as f64,
                height as f64,
                format!("data:{mime};base64,{base64}"),
            )
        }
        _ => {
            eprintln!("Error: {path} is not a png, a jpeg, a gif or a bmp");
            (0.0, 0.0, String::new())
        }
    }
}

/// The kind of the image and the size in its header, or `None` for anything
/// else. The header says the kind, and not the name of the file, so the data
/// URI always says what the bytes are.
fn image_header(data: &[u8]) -> Option<(&'static str, u32, u32)> {
    // PNG: bytes 16-23 contain width and height as u32 big-endian
    if data.len() >= 24 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some(("image/png", w, h));
    }
    // JPEG: walk the marker segments up to a start of frame, which is where the
    // dimensions are
    if data.len() >= 2 && data[0..2] == [0xFF, 0xD8] {
        let mut i = 2;
        while i + 1 < data.len() && data[i] == 0xFF {
            match data[i + 1] {
                // A marker may be padded with extra 0xFF bytes.
                0xFF => i += 1,
                // TEM, RST0-7, SOI and EOI carry no segment.
                0x01 | 0xD0..=0xD9 => i += 2,
                // Every kind of frame says its size the same way: length,
                // precision, height, width.
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                    if i + 9 > data.len() {
                        break;
                    }
                    let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                    let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                    return Some(("image/jpeg", w, h));
                }
                // The frame comes before the scan, so there is nothing ahead
                // but entropy-coded data.
                0xDA => break,
                _ => {
                    if i + 4 > data.len() {
                        break;
                    }
                    i += 2 + u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                }
            }
        }
    }
    // GIF: bytes 6-9 contain width and height as u16 little-endian
    if data.len() >= 10 && &data[0..4] == b"GIF8" {
        let w = u16::from_le_bytes([data[6], data[7]]) as u32;
        let h = u16::from_le_bytes([data[8], data[9]]) as u32;
        return Some(("image/gif", w, h));
    }
    // BMP: bytes 18-25 contain width and height as i32 little-endian
    if data.len() >= 26 && &data[0..2] == b"BM" {
        let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]).unsigned_abs();
        let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
        return Some(("image/bmp", w, h));
    }
    None
}

#[cfg(test)]
mod bitmap_tests {
    use super::image_header;

    #[test]
    fn png_dimensions() {
        // Minimal 1x1 PNG header
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0; 8]); // chunk length + type (IHDR)
        data.extend_from_slice(&10u32.to_be_bytes()); // width
        data.extend_from_slice(&20u32.to_be_bytes()); // height
        assert_eq!(image_header(&data), Some(("image/png", 10, 20)));
    }

    #[test]
    fn jpeg_dimensions() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, b'J', b'F']); // APP0
        data.extend_from_slice(&[0xFF, 0xFF]); // fill bytes before the marker
        data.extend_from_slice(&[0xFF, 0xC1, 0x00, 0x0B, 8]); // SOF1
        data.extend_from_slice(&70u16.to_be_bytes()); // height
        data.extend_from_slice(&80u16.to_be_bytes()); // width
        assert_eq!(image_header(&data), Some(("image/jpeg", 80, 70)));
    }

    #[test]
    fn jpeg_marker_inside_a_segment_is_data() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        // An APP0 whose payload happens to spell a SOF0
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x06, 0xFF, 0xC0, 0x00, 0x00]);
        data.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 8]); // SOF0
        data.extend_from_slice(&10u16.to_be_bytes()); // height
        data.extend_from_slice(&20u16.to_be_bytes()); // width
        assert_eq!(image_header(&data), Some(("image/jpeg", 20, 10)));
    }

    #[test]
    fn truncated_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B];
        assert_eq!(image_header(&data), None);
    }

    #[test]
    fn gif_dimensions() {
        let mut data = b"GIF89a".to_vec();
        data.extend_from_slice(&30u16.to_le_bytes()); // width
        data.extend_from_slice(&40u16.to_le_bytes()); // height
        assert_eq!(image_header(&data), Some(("image/gif", 30, 40)));
    }

    #[test]
    fn bmp_dimensions() {
        let mut data = vec![0; 26];
        data[0] = b'B';
        data[1] = b'M';
        data[18..22].copy_from_slice(&50u32.to_le_bytes()); // width
        data[22..26].copy_from_slice(&60u32.to_le_bytes()); // height
        assert_eq!(image_header(&data), Some(("image/bmp", 50, 60)));
    }

    #[test]
    fn bmp_negative_height() {
        let mut data = vec![0; 26];
        data[0] = b'B';
        data[1] = b'M';
        data[18..22].copy_from_slice(&50i32.to_le_bytes());
        data[22..26].copy_from_slice(&(-60i32).to_le_bytes()); // top-down
        assert_eq!(image_header(&data), Some(("image/bmp", 50, 60)));
    }

    #[test]
    fn empty_data() {
        assert_eq!(image_header(&[]), None);
    }

    #[test]
    fn invalid_data() {
        assert_eq!(image_header(b"not an image"), None);
    }

    #[test]
    fn truncated_png() {
        let data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        // Only header magic, no IHDR
        assert_eq!(image_header(&data), None);
    }
}

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
mod text_tests {
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
