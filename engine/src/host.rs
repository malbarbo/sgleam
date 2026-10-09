//! What a program asks of the world around it. The page answers in `wasm` and
//! the operating system in `native`.
//!
//! Each target provides every function re-exported below, under the name and
//! with the meaning written here. A `cfg` on a re-export says that only one
//! target has the function, and whatever asks for it says the same `cfg`.

use sinteract::scene::FontStyle;

/// Makes the world ready to answer. An engine calls it as it is built. The
/// first call does the work and a later one gives back its result.
pub use target::init;

/// Returns `true` if an interruption is waiting, `false` otherwise. Reading the
/// request clears it, so the run after an interrupted one does not stop at its
/// first check.
pub use target::check_interrupt;

/// `system.sleep` holds the program for `ms` milliseconds.
pub use target::sleep;

/// `system.load_bitmap` reads an image file and gives the width and the height
/// of the image, and its bytes as a data URI, which a drawing puts in an
/// `<image>` element. Zeros and an empty string say that the file is missing,
/// or that nothing here reads a file of that kind.
pub use target::load_bitmap;

/// `system.draw_svg` puts a drawing on the page. Only the browser draws, so a
/// native run has no such function.
#[cfg(target_arch = "wasm32")]
pub use target::draw_svg;

/// `system.get_key_event` gives the kind of the event waiting in the page, the
/// name of the key and the modifiers that are on, or nothing when no event
/// waits. Only the browser reads keys, so a native run has no such function.
#[cfg(target_arch = "wasm32")]
pub use target::get_key_event;

#[cfg(target_arch = "wasm32")]
#[path = "host/wasm.rs"]
mod target;
#[cfg(not(target_arch = "wasm32"))]
#[path = "host/native.rs"]
mod target;

/// `system.text_metrics` gives the width and the height of the box around the
/// text. sinteract measures, with the fonts that it embeds, so
/// a text has the same size on every target. A size that cannot draw measures
/// nothing.
pub fn text_metrics(text: String, font_css: String) -> (f64, f64) {
    let font = CssFont::parse(&font_css);
    match sinteract::text::measure(font.family, font.weight, font.style, font.size, &text) {
        Some(m) => (f64::from(m.width()), f64::from(m.height())),
        None => (0.0, 0.0),
    }
}

/// The font of a css shorthand from `font.to_css`, which writes the style, the
/// weight, the size in pixels and the family, in that order.
struct CssFont<'a> {
    style: FontStyle,
    weight: u16,
    size: f32,
    family: &'a str,
}

impl<'a> CssFont<'a> {
    /// Without a size the font is the 14 pixels of `font.default`.
    fn parse(css: &'a str) -> Self {
        let mut font = CssFont {
            style: FontStyle::Normal,
            weight: 400,
            size: 14.0,
            family: "",
        };
        let mut rest = css.trim_start();
        while let Some((word, after)) = rest.split_once(' ') {
            match word {
                "italic" => font.style = FontStyle::Italic,
                "oblique" => font.style = FontStyle::Oblique,
                "bold" => font.weight = 700,
                "lighter" => font.weight = 300,
                _ => {
                    if let Some(size) = word.strip_suffix("px").and_then(|d| d.parse().ok()) {
                        font.size = size;
                        font.family = after;
                    }
                    break;
                }
            }
            rest = after.trim_start();
        }
        font
    }
}

/// `system.now_ms` gives milliseconds since the epoch, and `world` times its
/// ticks by it.
pub fn now_ms() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_font_reads_the_style_the_weight_the_size_and_the_family() {
        let font = CssFont::parse("italic bold 20px Liberation Serif");
        assert_eq!(font.style, FontStyle::Italic);
        assert_eq!(font.weight, 700);
        assert_eq!(font.size, 20.0);
        assert_eq!(font.family, "Liberation Serif");
    }

    #[test]
    fn css_font_without_a_style_and_a_weight_is_normal() {
        let font = CssFont::parse("14.5px sans-serif");
        assert_eq!(font.style, FontStyle::Normal);
        assert_eq!(font.weight, 400);
        assert_eq!(font.size, 14.5);
        assert_eq!(font.family, "sans-serif");
    }

    #[test]
    fn text_metrics_measures_a_text() {
        let (width, height) = text_metrics("sgleam".into(), "14px sans-serif".into());
        assert!(width > 0.0 && height > 0.0);
    }

    #[test]
    fn text_metrics_of_a_size_that_cannot_draw_is_nothing() {
        let metrics = text_metrics("sgleam".into(), "0px sans-serif".into());
        assert_eq!(metrics, (0.0, 0.0));
    }
}
