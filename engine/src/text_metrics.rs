use crate::fonts::FONTDB;

struct FontProps {
    family: String,
    size: f64,
    style: String,
    weight: String,
}

fn parse_font_css(font_css: &str) -> FontProps {
    let mut style = "normal".to_string();
    let mut weight = "normal".to_string();
    let mut size = 14.0;
    let mut family = "sans-serif".to_string();

    let parts: Vec<&str> = font_css.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == "italic" || parts[i] == "oblique" {
            style = parts[i].to_string();
        } else if parts[i] == "bold" || parts[i] == "lighter" {
            weight = parts[i].to_string();
        } else if let Some(s) = parts[i].strip_suffix("px") {
            if let Ok(v) = s.parse::<f64>() {
                size = v;
                // Everything after the size is the family
                family = parts[i + 1..].join(" ");
                break;
            }
        }
        i += 1;
    }
    FontProps {
        family,
        size,
        style,
        weight,
    }
}

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
        fontdb: FONTDB.clone(),
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

fn parse_size(font_css: &str) -> f64 {
    font_css
        .split_whitespace()
        .find_map(|s| s.strip_suffix("px").and_then(|n| n.parse().ok()))
        .unwrap_or(14.0)
}

fn heuristic(text: &str, font_css: &str) -> (f64, f64, f64, f64) {
    let size = parse_size(font_css);
    let width = text.len() as f64 * size * 0.6;
    (width, size, 0.0, 0.0)
}

pub fn text_width(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).0
}

pub fn text_height(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).1
}

pub fn text_x_offset(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).2
}

pub fn text_y_offset(text: String, font_css: String) -> f64 {
    measure(&text, &font_css).3
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
    fn parse_font_css_bold_italic() {
        let fp = parse_font_css("italic bold 16px monospace");
        assert_eq!(fp.size, 16.0);
        assert_eq!(fp.family, "monospace");
        assert_eq!(fp.style, "italic");
        assert_eq!(fp.weight, "bold");
    }
}
