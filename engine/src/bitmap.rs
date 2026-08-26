//! `system.load_bitmap` reads an image file and gives a program the width and
//! the height of the image, and its bytes as a data URI, which a drawing puts
//! in an `<image>` element. Zeros and an empty string say that the file is
//! missing, or that nothing here reads a file of that kind.

#[cfg(target_arch = "wasm32")]
mod ffi {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        /// Fetch bitmap, cache it, return data URI length (0 on error).
        pub fn load_bitmap_fetch(path: *const u8, path_len: usize) -> usize;
        /// Read cached width/height.
        pub fn load_bitmap_width() -> f64;
        pub fn load_bitmap_height() -> f64;
        /// Copy cached data URI into buf. Returns bytes written.
        pub fn load_bitmap_data(buf: *mut u8, buf_len: usize) -> usize;
    }
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(not(target_arch = "wasm32"))]
pub fn load_bitmap(path: String) -> (f64, f64, String) {
    use std::path::Path;
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading {path}: {e}");
            return (0.0, 0.0, String::new());
        }
    };
    let (w, h) = image_dimensions(&data);
    if w == 0 || h == 0 {
        eprintln!("Error: could not detect image dimensions for {path}");
        return (0.0, 0.0, String::new());
    }
    let extension = Path::new(&path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let mime = match extension.as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    let data_uri = format!("data:{mime};base64,{b64}");
    (w as f64, h as f64, data_uri)
}

/// The size in the header of the image, or `(0, 0)` for anything else.
#[cfg(not(target_arch = "wasm32"))]
fn image_dimensions(data: &[u8]) -> (u32, u32) {
    // PNG: bytes 16-23 contain width and height as u32 big-endian
    if data.len() >= 24 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return (w, h);
    }
    // JPEG: walk the marker segments up to a start of frame, which is where
    // the dimensions are
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
                    return (w, h);
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
        return (w, h);
    }
    // BMP: bytes 18-25 contain width and height as i32 little-endian
    if data.len() >= 26 && &data[0..2] == b"BM" {
        let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]).unsigned_abs();
        let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
        return (w, h);
    }
    (0, 0)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::image_dimensions;

    #[test]
    fn png_dimensions() {
        // Minimal 1x1 PNG header
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0; 8]); // chunk length + type (IHDR)
        data.extend_from_slice(&10u32.to_be_bytes()); // width
        data.extend_from_slice(&20u32.to_be_bytes()); // height
        assert_eq!(image_dimensions(&data), (10, 20));
    }

    #[test]
    fn jpeg_dimensions() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, b'J', b'F']); // APP0
        data.extend_from_slice(&[0xFF, 0xFF]); // fill bytes before the marker
        data.extend_from_slice(&[0xFF, 0xC1, 0x00, 0x0B, 8]); // SOF1
        data.extend_from_slice(&70u16.to_be_bytes()); // height
        data.extend_from_slice(&80u16.to_be_bytes()); // width
        assert_eq!(image_dimensions(&data), (80, 70));
    }

    #[test]
    fn jpeg_marker_inside_a_segment_is_data() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        // An APP0 whose payload happens to spell a SOF0
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x06, 0xFF, 0xC0, 0x00, 0x00]);
        data.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 8]); // SOF0
        data.extend_from_slice(&10u16.to_be_bytes()); // height
        data.extend_from_slice(&20u16.to_be_bytes()); // width
        assert_eq!(image_dimensions(&data), (20, 10));
    }

    #[test]
    fn truncated_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B];
        assert_eq!(image_dimensions(&data), (0, 0));
    }

    #[test]
    fn gif_dimensions() {
        let mut data = b"GIF89a".to_vec();
        data.extend_from_slice(&30u16.to_le_bytes()); // width
        data.extend_from_slice(&40u16.to_le_bytes()); // height
        assert_eq!(image_dimensions(&data), (30, 40));
    }

    #[test]
    fn bmp_dimensions() {
        let mut data = vec![0; 26];
        data[0] = b'B';
        data[1] = b'M';
        data[18..22].copy_from_slice(&50u32.to_le_bytes()); // width
        data[22..26].copy_from_slice(&60u32.to_le_bytes()); // height
        assert_eq!(image_dimensions(&data), (50, 60));
    }

    #[test]
    fn bmp_negative_height() {
        let mut data = vec![0; 26];
        data[0] = b'B';
        data[1] = b'M';
        data[18..22].copy_from_slice(&50i32.to_le_bytes());
        data[22..26].copy_from_slice(&(-60i32).to_le_bytes()); // top-down
        assert_eq!(image_dimensions(&data), (50, 60));
    }

    #[test]
    fn empty_data() {
        assert_eq!(image_dimensions(&[]), (0, 0));
    }

    #[test]
    fn invalid_data() {
        assert_eq!(image_dimensions(b"not an image"), (0, 0));
    }

    #[test]
    fn truncated_png() {
        let data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        // Only header magic, no IHDR
        assert_eq!(image_dimensions(&data), (0, 0));
    }
}
