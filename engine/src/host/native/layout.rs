//! resvg lays out the text and gives the box around it. Parsing the css
//! is ours, since resvg wants the parts of a font one at a time.

use super::{font_size, heuristic};

mod fonts;

struct FontProps {
    family: String,
    size: f64,
    style: String,
    weight: String,
}

fn parse_font_css(font_css: &str) -> FontProps {
    let parts: Vec<&str> = font_css.split_whitespace().collect();
    let (size, size_at) = font_size(&parts);
    // The style and the weight come before the size and the family after it.
    // With no size at all, every part is read for a style and a weight, and
    // none is left to name a family.
    let size_at = size_at.unwrap_or(parts.len());
    let mut style = "normal".to_string();
    let mut weight = "normal".to_string();
    for part in parts.iter().take(size_at) {
        if *part == "italic" || *part == "oblique" {
            style = part.to_string();
        } else if *part == "bold" || *part == "lighter" {
            weight = part.to_string();
        }
    }
    let family = match parts
        .iter()
        .skip(size_at + 1)
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
    {
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

/// Asks resvg for the box around the text, and falls back to the guess when
/// resvg lays out nothing, which happens when the font database has no such
/// family.
pub fn measure(text: &str, font_css: &str) -> (f64, f64, f64, f64) {
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
        fontdb: fonts::FONTDB.clone(),
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

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_returns_positive_dimensions() {
        let (w, h, _, _) = measure("Hello", "24px sans-serif");
        assert!(w > 0.0, "width should be positive, got {w}");
        assert!(h > 0.0, "height should be positive, got {h}");
    }

    #[test]
    fn bold_is_wider_than_normal() {
        let (w_normal, _, _, _) = measure("Test", "24px sans-serif");
        let (w_bold, _, _, _) = measure("Test", "bold 24px sans-serif");
        assert!(
            w_bold > w_normal,
            "bold ({w_bold}) should be wider than normal ({w_normal})"
        );
    }

    #[test]
    fn larger_font_is_wider() {
        let (w_small, _, _, _) = measure("Test", "12px sans-serif");
        let (w_large, _, _, _) = measure("Test", "24px sans-serif");
        assert!(
            w_large > w_small,
            "24px ({w_large}) should be wider than 12px ({w_small})"
        );
    }

    #[test]
    fn x_offset_is_negative_half_width() {
        let (w, _, x_off, _) = measure("Hello", "24px sans-serif");
        assert!(
            (x_off + w / 2.0).abs() < 0.001,
            "x_offset ({x_off}) should be -width/2 (-{})",
            w / 2.0
        );
    }

    #[test]
    fn height_includes_descenders() {
        let (_, h_no_desc, _, _) = measure("HELLO", "24px sans-serif");
        let (_, h_desc, _, _) = measure("gypsy", "24px sans-serif");
        assert_eq!(
            h_no_desc, h_desc,
            "height should be consistent (font metrics, not glyph-specific)"
        );
    }

    #[test]
    fn parse_font_css_normal() {
        let fp = parse_font_css("24px sans-serif");
        assert_eq!(fp.size, 24.0);
        assert_eq!(fp.family, "sans-serif");
        assert_eq!(fp.style, "normal");
        assert_eq!(fp.weight, "normal");
    }

    #[test]
    fn parse_font_css_no_family() {
        // The size is the last part, or there is no size to leave one after.
        assert_eq!(parse_font_css("16px").family, "sans-serif");
        assert_eq!(parse_font_css("bold").family, "sans-serif");
        // And a family of several words is named by all of them.
        assert_eq!(parse_font_css("16px Fira Sans").family, "Fira Sans");
    }

    #[test]
    fn parse_font_css_bold_italic() {
        let fp = parse_font_css("italic bold 16px monospace");
        assert_eq!(fp.size, 16.0);
        assert_eq!(fp.family, "monospace");
        assert_eq!(fp.style, "italic");
        assert_eq!(fp.weight, "bold");
    }
}
