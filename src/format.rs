use colored::{Color, Colorize};

/// Apply color if not running under test, otherwise return the string
/// unchanged.  This is the single chokepoint for ANSI control — every
/// color helper routes through it.
fn apply_color(s: &str, color: Color) -> String {
    if cfg!(test) {
        s.to_string()
    } else {
        s.color(color).to_string()
    }
}

pub fn green_string(s: &str) -> String {
    apply_color(s, Color::Green)
}

pub fn red_string(s: &str) -> String {
    apply_color(s, Color::Red)
}

pub fn yellow_string(s: &str) -> String {
    apply_color(s, Color::Yellow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn green_string_is_plain() {
        assert_eq!(green_string("✓"), "✓");
    }

    #[test]
    fn red_string_is_plain() {
        assert_eq!(red_string("error"), "error");
    }

    #[test]
    fn yellow_string_is_plain() {
        assert_eq!(yellow_string("io"), "io");
    }

    #[test]
    fn all_helpers_strip_ansi_under_test() {
        // cfg!(test) is true here; all should return plain strings
        for input in &["hello", "✓", "error text"] {
            assert!(!green_string(input).contains('\x1b'));
            assert!(!red_string(input).contains('\x1b'));
            assert!(!yellow_string(input).contains('\x1b'));
        }
    }
}
