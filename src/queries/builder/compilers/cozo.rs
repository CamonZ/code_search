//! Cozo Datalog backend compiler.
//!
//! Implements `BackendCompiler` for Cozo, which uses:
//! - `$param_name` syntax for parameter placeholders
//! - Double-quoted string literals with backslash escaping

use super::BackendCompiler;

/// Cozo backend compiler implementation.
#[derive(Debug, Clone, Copy)]
pub struct CozoCompiler;

impl BackendCompiler for CozoCompiler {
    fn parameter_placeholder(&self, name: &str) -> String {
        format!("${}", name)
    }

    fn escape_string(&self, s: &str) -> String {
        // Cozo uses double quotes with backslash escaping
        format!("\"{}\"", s.replace("\\", "\\\\").replace("\"", "\\\""))
    }

    fn compile_filter(&self, field: &str, op: &str, param: &str) -> String {
        format!("{} {} {}", field, op, param)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_placeholder() {
        let compiler = CozoCompiler;
        assert_eq!(compiler.parameter_placeholder("limit"), "$limit");
        assert_eq!(compiler.parameter_placeholder("module"), "$module");
    }

    #[test]
    fn test_escape_string_simple() {
        let compiler = CozoCompiler;
        let escaped = compiler.escape_string("hello");
        assert_eq!(escaped, "\"hello\"");
    }

    #[test]
    fn test_escape_string_with_quotes() {
        let compiler = CozoCompiler;
        let escaped = compiler.escape_string("hello \"world\"");
        assert_eq!(escaped, "\"hello \\\"world\\\"\"");
    }

    #[test]
    fn test_escape_string_with_backslash() {
        let compiler = CozoCompiler;
        let escaped = compiler.escape_string("path\\to\\file");
        assert_eq!(escaped, "\"path\\\\to\\\\file\"");
    }

    #[test]
    fn test_escape_string_with_both() {
        let compiler = CozoCompiler;
        let escaped = compiler.escape_string("path\\with\"quotes");
        assert_eq!(escaped, "\"path\\\\with\\\"quotes\"");
    }

    #[test]
    fn test_compile_filter() {
        let compiler = CozoCompiler;
        let filter = compiler.compile_filter("field", "==", "$value");
        assert_eq!(filter, "field == $value");
    }

    #[test]
    fn test_compile_filter_with_different_operators() {
        let compiler = CozoCompiler;
        assert_eq!(
            compiler.compile_filter("x", ">", "$limit"),
            "x > $limit"
        );
        assert_eq!(
            compiler.compile_filter("y", "<", "$threshold"),
            "y < $threshold"
        );
        assert_eq!(
            compiler.compile_filter("z", "!=", "$value"),
            "z != $value"
        );
    }
}
