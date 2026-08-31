//! The file system holds an image. Its own header says what kind it is and how
//! big, and the bytes go on to the drawing as a data URI.

use base64::Engine as _;

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

/// The `N` bytes at `at`, or `None` when the data stops before them.
fn bytes<const N: usize>(data: &[u8], at: usize) -> Option<[u8; N]> {
    data.get(at..at + N)?.try_into().ok()
}

/// The kind of the image and the size in its header, or `None` for anything
/// else. The header says the kind, and not the name of the file, so the data
/// URI always says what the bytes are.
fn image_header(data: &[u8]) -> Option<(&'static str, u32, u32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        let w = u32::from_be_bytes(bytes(data, 16)?);
        let h = u32::from_be_bytes(bytes(data, 20)?);
        return Some(("image/png", w, h));
    }
    // JPEG: walk the marker segments up to a start of frame, which is where the
    // dimensions are
    if data.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while data.get(i) == Some(&0xFF)
            && let Some(&marker) = data.get(i + 1)
        {
            match marker {
                // A marker may be padded with extra 0xFF bytes.
                0xFF => i += 1,
                // TEM, RST0-7, SOI and EOI carry no segment.
                0x01 | 0xD0..=0xD9 => i += 2,
                // Every kind of frame says its size the same way: length,
                // precision, height, width.
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                    let h = u16::from_be_bytes(bytes(data, i + 5)?) as u32;
                    let w = u16::from_be_bytes(bytes(data, i + 7)?) as u32;
                    return Some(("image/jpeg", w, h));
                }
                // The frame comes before the scan, so there is nothing ahead
                // but entropy-coded data.
                0xDA => break,
                _ => i += 2 + u16::from_be_bytes(bytes(data, i + 2)?) as usize,
            }
        }
    }
    if data.starts_with(b"GIF8") {
        let w = u16::from_le_bytes(bytes(data, 6)?) as u32;
        let h = u16::from_le_bytes(bytes(data, 8)?) as u32;
        return Some(("image/gif", w, h));
    }
    if data.starts_with(b"BM") {
        let w = i32::from_le_bytes(bytes(data, 18)?).unsigned_abs();
        let h = i32::from_le_bytes(bytes(data, 22)?).unsigned_abs();
        return Some(("image/bmp", w, h));
    }
    None
}

// A test writes the header it means to read back, and the buffer is right
// there, the size it wrote.
#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
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
