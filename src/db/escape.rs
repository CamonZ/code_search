//! String escaping utilities for CozoDB queries.

/// Escape a string for use in CozoDB string literals.
///
/// # Arguments
/// * `s` - The string to escape
/// * `quote_char` - The quote character to escape ('"' for double-quoted, '\'' for single-quoted)
pub fn escape_string_for_quote(s: &str, quote_char: char) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            c if c == quote_char => {
                result.push('\\');
                result.push(c);
            }
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() || c == '\0' => {
                // Escape control characters as \uXXXX (JSON format)
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// Escape a string for use in CozoDB double-quoted string literals (JSON-compatible)
#[inline]
pub fn escape_string(s: &str) -> String {
    escape_string_for_quote(s, '"')
}

/// Escape a string for use in CozoDB single-quoted string literals.
/// Use this for strings that may contain double quotes or complex content.
#[inline]
pub fn escape_string_single(s: &str) -> String {
    escape_string_for_quote(s, '\'')
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn test_escape_string_basic() {
        assert_eq!(escape_string("hello"), "hello");
    }

    #[rstest]
    fn test_escape_string_with_quotes() {
        assert_eq!(escape_string(r#"say "hello""#), r#"say \"hello\""#);
    }

    #[rstest]
    fn test_escape_string_with_backslash() {
        assert_eq!(escape_string(r"path\to\file"), r"path\\to\\file");
    }
}
