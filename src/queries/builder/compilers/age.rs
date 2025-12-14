//! PostgreSQL Apache Age (AGE) backend compiler.
//!
//! Implements `BackendCompiler` for AGE, which uses:
//! - `$param_name` syntax for parameter placeholders (via pgvector)
//! - Single-quoted string literals with backslash escaping
//! - Cypher query syntax

use super::BackendCompiler;

/// AGE backend compiler implementation.
#[derive(Debug, Clone, Copy)]
pub struct AgeCompiler;

impl BackendCompiler for AgeCompiler {
    fn parameter_placeholder(&self, name: &str) -> String {
        // AGE/Cypher uses parameterized queries with $param syntax
        format!("${}", name)
    }

    fn escape_string(&self, s: &str) -> String {
        // AGE Cypher uses single quotes with backslash escaping
        format!("'{}'", s.replace("\\", "\\\\").replace("'", "\\'"))
    }

    fn compile_filter(&self, property: &str, op: &str, param: &str) -> String {
        // In Cypher, filters are typically: m.property op $param
        format!("{} {} {}", property, op, param)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_placeholder() {
        let compiler = AgeCompiler;
        assert_eq!(compiler.parameter_placeholder("limit"), "$limit");
        assert_eq!(compiler.parameter_placeholder("module"), "$module");
    }

    #[test]
    fn test_escape_string_simple() {
        let compiler = AgeCompiler;
        let escaped = compiler.escape_string("hello");
        assert_eq!(escaped, "'hello'");
    }

    #[test]
    fn test_escape_string_with_single_quotes() {
        let compiler = AgeCompiler;
        let escaped = compiler.escape_string("it's");
        assert_eq!(escaped, "'it\\'s'");
    }

    #[test]
    fn test_escape_string_with_backslash() {
        let compiler = AgeCompiler;
        let escaped = compiler.escape_string("path\\to\\file");
        assert_eq!(escaped, "'path\\\\to\\\\file'");
    }

    #[test]
    fn test_escape_string_with_both() {
        let compiler = AgeCompiler;
        let escaped = compiler.escape_string("path\\with'quotes");
        assert_eq!(escaped, "'path\\\\with\\'quotes'");
    }

    #[test]
    fn test_compile_filter() {
        let compiler = AgeCompiler;
        let filter = compiler.compile_filter("m.name", "==", "$value");
        assert_eq!(filter, "m.name == $value");
    }

    #[test]
    fn test_compile_filter_with_different_operators() {
        let compiler = AgeCompiler;
        assert_eq!(
            compiler.compile_filter("m.count", ">", "$limit"),
            "m.count > $limit"
        );
        assert_eq!(
            compiler.compile_filter("m.score", "<", "$threshold"),
            "m.score < $threshold"
        );
        assert_eq!(
            compiler.compile_filter("m.status", "!=", "$value"),
            "m.status != $value"
        );
    }
}
