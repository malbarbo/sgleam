//! How much room a piece of text takes, which is what `system.text_width` and
//! its neighbours give a program: the width and height of the box around the
//! text, and the offsets from the middle of that box to the origin an svg
//! `<text>` element takes, which is the start of the baseline.
//!
//! The page measures the text on wasm32. Natively resvg does, or, without the
//! `resvg` feature, the size of the font gives a rough guess.

#[cfg(target_arch = "wasm32")]
mod ffi {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
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

#[cfg(target_arch = "wasm32")]
pub fn text_width(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_width(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}

#[cfg(target_arch = "wasm32")]
pub fn text_height(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_height(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}

#[cfg(target_arch = "wasm32")]
pub fn text_x_offset(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_x_offset(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}

#[cfg(target_arch = "wasm32")]
pub fn text_y_offset(text: String, font_css: String) -> f64 {
    unsafe { ffi::text_y_offset(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn text_width(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).0
}

#[cfg(not(target_arch = "wasm32"))]
pub fn text_height(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).1
}

#[cfg(not(target_arch = "wasm32"))]
pub fn text_x_offset(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).2
}

#[cfg(not(target_arch = "wasm32"))]
pub fn text_y_offset(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).3
}

/// The size in pixels, and which part of the css says it: the part that ends
/// in `px`. The parts before it say the style and the weight, and the parts
/// after it name the family. Without such a part the size is 14.
#[cfg(not(target_arch = "wasm32"))]
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

/// A character as wide as a fixed part of the size of the font, and a line as
/// tall as the size, which is close enough to lay a drawing out with.
#[cfg(not(target_arch = "wasm32"))]
fn heuristic(text: &str, font_css: &str) -> (f64, f64, f64, f64) {
    let (size, _) = font_size(&font_css.split_whitespace().collect::<Vec<_>>());
    let width = text.chars().count() as f64 * size * 0.6;
    (width, size, 0.0, 0.0)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "resvg"))]
struct FontProps {
    family: String,
    size: f64,
    style: String,
    weight: String,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "resvg"))]
fn parse_font_css(font_css: &str) -> FontProps {
    let parts: Vec<&str> = font_css.split_whitespace().collect();
    let (size, size_at) = font_size(&parts);
    let mut style = "normal".to_string();
    let mut weight = "normal".to_string();
    for part in &parts[..size_at.unwrap_or(parts.len())] {
        if *part == "italic" || *part == "oblique" {
            style = part.to_string();
        } else if *part == "bold" || *part == "lighter" {
            weight = part.to_string();
        }
    }
    let family = match parts[size_at.map_or(parts.len(), |at| at + 1)..].join(" ") {
        family if family.is_empty() => "sans-serif".to_string(),
        family => family,
    };
    FontProps {
        family,
        size,
        style,
        weight,
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "resvg")))]
fn measure(text: &str, font_css: &str) -> (f64, f64, f64, f64) {
    heuristic(text, font_css)
}

/// The box resvg lays the text out in, or the guess when it lays out nothing:
/// a family the font database does not have leaves an empty box.
#[cfg(all(not(target_arch = "wasm32"), feature = "resvg"))]
fn measure(text: &str, font_css: &str) -> (f64, f64, f64, f64) {
    let fp = parse_font_css(font_css);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><text font-family="{}" font-size="{}" font-style="{}" font-weight="{}" x="0" y="0">{}</text></svg>"#,
        fp.family,
        fp.size,
        fp.style,
        fp.weight,
        html_escape(text)
    );
    let opts = resvg::usvg::Options {
        fontdb: crate::fonts::FONTDB.clone(),
        ..Default::default()
    };
    let tree = match resvg::usvg::Tree::from_str(&svg, &opts) {
        Ok(t) => t,
        Err(_) => return heuristic(text, font_css),
    };
    let bbox = tree.root().bounding_box();
    if bbox.width() <= 0.0 || bbox.height() <= 0.0 {
        return heuristic(text, font_css);
    }
    let width = bbox.width() as f64;
    let height = bbox.height() as f64;
    let ascent = -bbox.top() as f64;
    let x_offset = -width / 2.0;
    let y_offset = ascent - height / 2.0;
    (width, height, x_offset, y_offset)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "resvg"))]
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[cfg(feature = "resvg")]
    #[test]
    fn measure_returns_positive_dimensions() {
        let (w, h, _, _) = measure("Hello", "24px sans-serif");
        assert!(w > 0.0, "width should be positive, got {w}");
        assert!(h > 0.0, "height should be positive, got {h}");
    }

    #[cfg(feature = "resvg")]
    #[test]
    fn bold_is_wider_than_normal() {
        let (w_normal, _, _, _) = measure("Test", "24px sans-serif");
        let (w_bold, _, _, _) = measure("Test", "bold 24px sans-serif");
        assert!(
            w_bold > w_normal,
            "bold ({w_bold}) should be wider than normal ({w_normal})"
        );
    }

    #[cfg(feature = "resvg")]
    #[test]
    fn larger_font_is_wider() {
        let (w_small, _, _, _) = measure("Test", "12px sans-serif");
        let (w_large, _, _, _) = measure("Test", "24px sans-serif");
        assert!(
            w_large > w_small,
            "24px ({w_large}) should be wider than 12px ({w_small})"
        );
    }

    #[cfg(feature = "resvg")]
    #[test]
    fn x_offset_is_negative_half_width() {
        let (w, _, x_off, _) = measure("Hello", "24px sans-serif");
        assert!(
            (x_off + w / 2.0).abs() < 0.001,
            "x_offset ({x_off}) should be -width/2 (-{})",
            w / 2.0
        );
    }

    #[cfg(feature = "resvg")]
    #[test]
    fn height_includes_descenders() {
        let (_, h_no_desc, _, _) = measure("HELLO", "24px sans-serif");
        let (_, h_desc, _, _) = measure("gypsy", "24px sans-serif");
        assert_eq!(
            h_no_desc, h_desc,
            "height should be consistent (font metrics, not glyph-specific)"
        );
    }

    #[cfg(feature = "resvg")]
    #[test]
    fn parse_font_css_normal() {
        let fp = parse_font_css("24px sans-serif");
        assert_eq!(fp.size, 24.0);
        assert_eq!(fp.family, "sans-serif");
        assert_eq!(fp.style, "normal");
        assert_eq!(fp.weight, "normal");
    }

    #[cfg(feature = "resvg")]
    #[test]
    fn parse_font_css_bold_italic() {
        let fp = parse_font_css("italic bold 16px monospace");
        assert_eq!(fp.size, 16.0);
        assert_eq!(fp.family, "monospace");
        assert_eq!(fp.style, "italic");
        assert_eq!(fp.weight, "bold");
    }

    #[test]
    fn a_css_with_no_size_in_pixels_has_the_default() {
        assert_eq!(font_size(&["bold", "16px", "monospace"]), (16.0, Some(1)));
        assert_eq!(font_size(&["bold", "large"]), (14.0, None));
    }

    #[test]
    fn the_guess_grows_with_the_text_and_the_size() {
        let (w, h, x, y) = heuristic("abc", "10px sans-serif");
        assert_eq!((h, x, y), (10.0, 0.0, 0.0));
        assert!(w > heuristic("ab", "10px sans-serif").0);
        assert!(heuristic("abc", "20px sans-serif").0 > w);
    }
}
