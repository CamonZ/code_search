//! Database connection and query utilities for SurrealDB.
//!
//! This module provides the database abstraction layer for the CLI tool:
//! - Connection management (file-backed or in-memory for tests)
//! - Query execution with parameter binding
//! - Result row extraction with type-safe helpers
//!
//! # Architecture
//!
//! SurrealDB is a multi-model database that stores call graph data in tables.
//! Queries are written in SurrealQL and return results that must be extracted
//! into Rust types.
//!
//! # Type Decisions
//!
//! **Why `i64` for arity/line numbers instead of `u32`?**
//! SurrealDB returns all integers as i64. Using `i64` throughout avoids
//! lossy conversions and potential panics. The semantic constraint (arity >= 0)
//! is enforced by the data source (Elixir AST), not runtime checks.
//!
//! **Why `CallRowLayout` with indices instead of serde deserialization?**
//! SurrealDB returns rows as ordered values. The `CallRowLayout` struct documents
//! column positions for each query type, centralizing the mapping in factory
//! methods rather than scattering magic numbers.
//!
//! **Why bare `String` for module/function names instead of newtypes?**
//! For a CLI tool, the complexity of newtype wrappers (`.0` access, `Into` impls,
//! derive macro limitations) outweighs the type safety benefit. Field names
//! (`module`, `name`) are sufficiently clear.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::rc::Rc;

use thiserror::Error;

use crate::backend::{Database, Row, Value};
use crate::types::{Call, FunctionRef};

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Failed to open database '{path}': {message}")]
    OpenFailed { path: String, message: String },

    #[error("Query failed: {message}")]
    QueryFailed { message: String },

    #[error("Missing column '{name}' in query result")]
    MissingColumn { name: String },
}

/// Open a database at the specified path.
///
/// Returns a trait object for backend-agnostic database access.
pub fn open_db(path: &Path) -> Result<Box<dyn Database>, Box<dyn Error>> {
    crate::backend::open_database(path)
}

/// Create an in-memory database instance.
///
/// Used for tests to avoid disk I/O and temp file management.
#[cfg(any(test, feature = "test-utils"))]
pub fn open_mem_db() -> Result<Box<dyn Database>, Box<dyn Error>> {
    crate::backend::open_mem_database()
}

/// Run a database query with parameters.
///
/// Works with any backend that implements the Database trait.
/// Accepts QueryParams for type-safe parameter binding.
/// Returns a trait object that provides access to query results.
pub fn run_query(
    db: &dyn Database,
    script: &str,
    params: crate::backend::QueryParams,
) -> Result<Box<dyn crate::backend::QueryResult>, Box<dyn Error>> {
    db.execute_query(script, params)
}

/// Run a database query with no parameters.
///
/// Convenience wrapper around run_query for queries without parameters.
pub fn run_query_no_params(
    db: &dyn Database,
    script: &str,
) -> Result<Box<dyn crate::backend::QueryResult>, Box<dyn Error>> {
    run_query(db, script, crate::backend::QueryParams::new())
}

/// Escape a string for use in string literals.
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

/// Escape a string for use in double-quoted string literals (JSON-compatible)
#[inline]
pub fn escape_string(s: &str) -> String {
    escape_string_for_quote(s, '"')
}

/// Escape a string for use in single-quoted string literals.
/// Use this for strings that may contain double quotes or complex content.
#[inline]
pub fn escape_string_single(s: &str) -> String {
    escape_string_for_quote(s, '\'')
}

/// Try to create a relation, returning Ok(true) if created, Ok(false) if already exists.
///
/// This function attempts to create a database relation/table. If the relation already
/// exists, it returns Ok(false) instead of failing.
///
/// Backend-specific error patterns:
/// - **SurrealDB**: Detects "already exists" and "already defined" errors
pub fn try_create_relation(db: &dyn Database, script: &str) -> Result<bool, Box<dyn Error>> {
    match run_query_no_params(db, script) {
        Ok(_) => Ok(true),
        Err(e) => {
            let err_str = e.to_string();

            // SurrealDB: Check for table already exists errors
            if err_str.contains("already exists") {
                return Ok(false);
            }

            // Genuine error - propagate
            Err(e)
        }
    }
}

// Trait-based extraction helpers

/// Extract a String from a Value trait object, returning None if not a string
pub fn extract_string(value: &dyn Value) -> Option<String> {
    value.as_str().map(|s| s.to_string())
}

/// Extract an i64 from a Value trait object, returning the default if not a number
pub fn extract_i64(value: &dyn Value, default: i64) -> i64 {
    value.as_i64().unwrap_or(default)
}

/// Extract a String from a Value trait object, returning the default if not a string
pub fn extract_string_or(value: &dyn Value, default: &str) -> String {
    value
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| default.to_string())
}

/// Extract a bool from a Value trait object, returning the default if not a bool
pub fn extract_bool(value: &dyn Value, default: bool) -> bool {
    value.as_bool().unwrap_or(default)
}

/// Extract an f64 from a Value trait object, returning the default if not a number
pub fn extract_f64(value: &dyn Value, default: f64) -> f64 {
    value.as_f64().unwrap_or(default)
}

/// Layout descriptor for extracting call data from query result rows
#[derive(Debug)]
pub struct CallRowLayout {
    pub caller_module_idx: usize,
    pub caller_name_idx: usize,
    pub caller_arity_idx: usize,
    pub caller_kind_idx: usize,
    pub caller_start_line_idx: usize,
    pub caller_end_line_idx: usize,
    pub callee_module_idx: usize,
    pub callee_name_idx: usize,
    pub callee_arity_idx: usize,
    pub file_idx: usize,
    pub line_idx: usize,
    pub call_type_idx: Option<usize>,
}

impl CallRowLayout {
    /// Build layout dynamically from query result headers.
    ///
    /// This looks up column positions by name, making queries resilient to
    /// column reordering. Returns error if any required column is missing.
    ///
    /// Expected column names (from SurrealQL queries):
    /// - caller_module, caller_name, caller_arity, caller_kind
    /// - caller_start_line, caller_end_line
    /// - callee_module, callee_function, callee_arity
    /// - file, call_line
    /// - call_type (optional)
    pub fn from_headers(headers: &[String]) -> Result<Self, DbError> {
        // Build lookup map once: O(m) where m = number of headers
        let header_map: HashMap<&str, usize> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| (h.as_str(), i))
            .collect();

        // Helper for required columns: O(1) each
        let find = |name: &str| -> Result<usize, DbError> {
            header_map
                .get(name)
                .copied()
                .ok_or_else(|| DbError::MissingColumn {
                    name: name.to_string(),
                })
        };

        Ok(Self {
            caller_module_idx: find("caller_module")?,
            caller_name_idx: find("caller_name")?,
            caller_arity_idx: find("caller_arity")?,
            caller_kind_idx: find("caller_kind")?,
            caller_start_line_idx: find("caller_start_line")?,
            caller_end_line_idx: find("caller_end_line")?,
            callee_module_idx: find("callee_module")?,
            callee_name_idx: find("callee_function")?,
            callee_arity_idx: find("callee_arity")?,
            file_idx: find("file")?,
            line_idx: find("call_line")?,
            call_type_idx: header_map.get("call_type").copied(),
        })
    }
}

/// Extract call data from a trait object row
///
/// Returns Option<Call> if all required fields are present. Uses early return
/// (None) if any required string field cannot be extracted. This version works
/// with the trait-based Row interface.
pub fn extract_call_from_row_trait(row: &dyn Row, layout: &CallRowLayout) -> Option<Call> {
    // Extract caller information
    let caller_module = row
        .get(layout.caller_module_idx)
        .and_then(|v| extract_string(v))?;
    let caller_name = row
        .get(layout.caller_name_idx)
        .and_then(|v| extract_string(v))?;
    let caller_arity = row
        .get(layout.caller_arity_idx)
        .map(|v| extract_i64(v, 0))
        .unwrap_or(0);
    let caller_kind = row
        .get(layout.caller_kind_idx)
        .map(|v| extract_string_or(v, ""))
        .unwrap_or_default();
    let caller_start_line = row
        .get(layout.caller_start_line_idx)
        .map(|v| extract_i64(v, 0))
        .unwrap_or(0);
    let caller_end_line = row
        .get(layout.caller_end_line_idx)
        .map(|v| extract_i64(v, 0))
        .unwrap_or(0);

    // Extract callee information
    let callee_module = row
        .get(layout.callee_module_idx)
        .and_then(|v| extract_string(v))?;
    let callee_name = row
        .get(layout.callee_name_idx)
        .and_then(|v| extract_string(v))?;
    let callee_arity = row
        .get(layout.callee_arity_idx)
        .map(|v| extract_i64(v, 0))
        .unwrap_or(0);

    // Extract file and line
    let file = row.get(layout.file_idx).and_then(|v| extract_string(v))?;
    let line = row
        .get(layout.line_idx)
        .map(|v| extract_i64(v, 0))
        .unwrap_or(0);

    // Extract optional call_type
    let call_type = layout
        .call_type_idx
        .and_then(|idx| row.get(idx).map(|v| extract_string_or(v, "remote")));

    // Create FunctionRef objects with Rc<str> to reduce memory allocations
    let caller = FunctionRef::with_definition(
        Rc::from(caller_module.into_boxed_str()),
        Rc::from(caller_name.into_boxed_str()),
        caller_arity,
        Rc::from(caller_kind.into_boxed_str()),
        Rc::from(file.into_boxed_str()),
        caller_start_line,
        caller_end_line,
    );

    let callee = FunctionRef::new(
        Rc::from(callee_module.into_boxed_str()),
        Rc::from(callee_name.into_boxed_str()),
        callee_arity,
    );

    // Return Call
    Some(Call {
        caller,
        callee,
        line,
        call_type,
        depth: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Mock Value and Row implementations for unit testing
    // =========================================================================

    /// A mock Value for unit tests that can hold any of the supported types.
    #[derive(Debug, Clone)]
    enum MockValue {
        Str(String),
        Int(i64),
        Float(f64),
        Bool(bool),
        /// Represents a value whose type doesn't match what the extractor expects
        None,
    }

    impl Value for MockValue {
        fn as_str(&self) -> Option<&str> {
            match self {
                MockValue::Str(s) => Some(s.as_str()),
                _ => None,
            }
        }

        fn as_i64(&self) -> Option<i64> {
            match self {
                MockValue::Int(i) => Some(*i),
                _ => None,
            }
        }

        fn as_f64(&self) -> Option<f64> {
            match self {
                MockValue::Float(f) => Some(*f),
                _ => None,
            }
        }

        fn as_bool(&self) -> Option<bool> {
            match self {
                MockValue::Bool(b) => Some(*b),
                _ => None,
            }
        }

        fn as_array(&self) -> Option<Vec<&dyn Value>> {
            None
        }

        fn as_thing_id(&self) -> Option<&dyn Value> {
            None
        }
    }

    /// A mock Row for unit tests that holds a vector of MockValues.
    struct MockRow {
        values: Vec<MockValue>,
    }

    impl MockRow {
        fn new(values: Vec<MockValue>) -> Self {
            Self { values }
        }
    }

    impl Row for MockRow {
        fn get(&self, index: usize) -> Option<&dyn Value> {
            self.values.get(index).map(|v| v as &dyn Value)
        }

        fn len(&self) -> usize {
            self.values.len()
        }
    }

    // =========================================================================
    // escape_string tests (existing)
    // =========================================================================

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

    // =========================================================================
    // escape_string_for_quote tests
    // =========================================================================

    #[rstest]
    fn test_escape_string_for_quote_normal_string_unchanged() {
        // Normal ASCII text should pass through unchanged
        assert_eq!(escape_string_for_quote("hello world", '"'), "hello world");
    }

    #[rstest]
    fn test_escape_string_for_quote_escapes_quote_char() {
        // The specified quote character must be escaped
        assert_eq!(
            escape_string_for_quote(r#"say "hi""#, '"'),
            r#"say \"hi\""#
        );
        assert_eq!(
            escape_string_for_quote("it's here", '\''),
            r"it\'s here"
        );
    }

    #[rstest]
    fn test_escape_string_for_quote_null_byte() {
        // Null byte (\0) must be escaped as \u0000
        let input = "before\0after";
        let result = escape_string_for_quote(input, '"');
        assert_eq!(result, "before\\u0000after");
    }

    #[rstest]
    fn test_escape_string_for_quote_control_chars() {
        // Control characters (other than \n, \r, \t) must be escaped as \uXXXX.
        // BEL (0x07) is a control char that is not \n, \r, or \t.
        let bel = '\x07';
        let input = format!("a{}b", bel);
        let result = escape_string_for_quote(&input, '"');
        assert_eq!(result, "a\\u0007b");
    }

    #[rstest]
    fn test_escape_string_for_quote_control_char_or_null_both_branches() {
        // This tests that the `||` in `c.is_control() || c == '\0'` works:
        // \x01 is a control char AND is not '\0' -- exercises `is_control()` path
        // \0 is both a control char and the explicit '\0' check
        let input_ctrl = "\x01";
        assert_eq!(
            escape_string_for_quote(input_ctrl, '"'),
            "\\u0001",
            "Control char (SOH) should be escaped"
        );

        let input_null = "\0";
        assert_eq!(
            escape_string_for_quote(input_null, '"'),
            "\\u0000",
            "Null byte should be escaped"
        );
    }

    #[rstest]
    fn test_escape_string_for_quote_newline_tab_cr() {
        // \n, \r, \t have their own dedicated escape sequences
        assert_eq!(escape_string_for_quote("a\nb", '"'), "a\\nb");
        assert_eq!(escape_string_for_quote("a\rb", '"'), "a\\rb");
        assert_eq!(escape_string_for_quote("a\tb", '"'), "a\\tb");
    }

    #[rstest]
    fn test_escape_string_for_quote_mixed_content() {
        // Mix of normal chars, quote chars, backslashes, control chars
        let input = "path\\to\x00\"file\"\n";
        let result = escape_string_for_quote(input, '"');
        assert_eq!(result, "path\\\\to\\u0000\\\"file\\\"\\n");
    }

    #[rstest]
    fn test_escape_string_for_quote_empty_string() {
        assert_eq!(escape_string_for_quote("", '"'), "");
        assert_eq!(escape_string_for_quote("", '\''), "");
    }

    // =========================================================================
    // escape_string_single tests
    // =========================================================================

    #[rstest]
    fn test_escape_string_single_normal_text() {
        // Normal text should pass through unchanged
        assert_eq!(escape_string_single("hello world"), "hello world");
    }

    #[rstest]
    fn test_escape_string_single_escapes_single_quotes() {
        // Single quotes must be escaped
        assert_eq!(escape_string_single("it's"), r"it\'s");
    }

    #[rstest]
    fn test_escape_string_single_preserves_double_quotes() {
        // Double quotes should NOT be escaped (only single quotes are)
        assert_eq!(escape_string_single(r#"say "hi""#), r#"say "hi""#);
    }

    #[rstest]
    fn test_escape_string_single_backslash() {
        assert_eq!(escape_string_single(r"a\b"), r"a\\b");
    }

    #[rstest]
    fn test_escape_string_single_control_chars() {
        assert_eq!(escape_string_single("a\nb"), "a\\nb");
        assert_eq!(escape_string_single("a\x07b"), "a\\u0007b");
    }

    #[rstest]
    fn test_escape_string_single_empty() {
        assert_eq!(escape_string_single(""), "");
    }

    #[rstest]
    fn test_escape_string_single_mixed() {
        // Mix of single quotes, backslashes, newlines
        let input = "it's a \\path\n";
        let result = escape_string_single(input);
        assert_eq!(result, r"it\'s a \\path\n");
    }

    // =========================================================================
    // extract_bool tests
    // =========================================================================

    #[rstest]
    fn test_extract_bool_true_value() {
        let val = MockValue::Bool(true);
        assert_eq!(extract_bool(&val, false), true);
    }

    #[rstest]
    fn test_extract_bool_false_value() {
        let val = MockValue::Bool(false);
        assert_eq!(extract_bool(&val, true), false);
    }

    #[rstest]
    fn test_extract_bool_missing_returns_default_true() {
        // When value is not a bool, default should be returned
        let val = MockValue::None;
        assert_eq!(extract_bool(&val, true), true);
    }

    #[rstest]
    fn test_extract_bool_missing_returns_default_false() {
        let val = MockValue::None;
        assert_eq!(extract_bool(&val, false), false);
    }

    #[rstest]
    fn test_extract_bool_wrong_type_returns_default() {
        // A string value is not a bool, should return default
        let val = MockValue::Str("true".to_string());
        assert_eq!(extract_bool(&val, false), false);
    }

    // =========================================================================
    // extract_f64 tests
    // =========================================================================

    #[rstest]
    fn test_extract_f64_positive_value() {
        let val = MockValue::Float(3.14);
        assert_eq!(extract_f64(&val, 0.0), 3.14);
    }

    #[rstest]
    fn test_extract_f64_negative_value() {
        let val = MockValue::Float(-2.5);
        assert_eq!(extract_f64(&val, 0.0), -2.5);
    }

    #[rstest]
    fn test_extract_f64_zero_value() {
        let val = MockValue::Float(0.0);
        // When the value IS 0.0, it should return 0.0 even if default is different
        assert_eq!(extract_f64(&val, 99.9), 0.0);
    }

    #[rstest]
    fn test_extract_f64_missing_returns_default() {
        let val = MockValue::None;
        assert_eq!(extract_f64(&val, 42.0), 42.0);
    }

    #[rstest]
    fn test_extract_f64_wrong_type_returns_default() {
        let val = MockValue::Str("3.14".to_string());
        assert_eq!(extract_f64(&val, -1.0), -1.0);
    }

    // =========================================================================
    // extract_call_from_row_trait tests
    // =========================================================================

    /// Build a standard layout matching the standard_headers() column order.
    fn standard_layout() -> CallRowLayout {
        CallRowLayout::from_headers(&standard_headers()).unwrap()
    }

    /// Build a MockRow with all 11 required fields populated for a valid call.
    fn valid_call_row() -> MockRow {
        // Column order: caller_module(0), caller_name(1), caller_arity(2),
        //   caller_kind(3), caller_start_line(4), caller_end_line(5),
        //   callee_module(6), callee_function(7), callee_arity(8),
        //   file(9), call_line(10)
        MockRow::new(vec![
            MockValue::Str("MyApp.Controller".to_string()), // caller_module
            MockValue::Str("index".to_string()),            // caller_name
            MockValue::Int(2),                              // caller_arity
            MockValue::Str("def".to_string()),              // caller_kind
            MockValue::Int(10),                             // caller_start_line
            MockValue::Int(30),                             // caller_end_line
            MockValue::Str("MyApp.Repo".to_string()),       // callee_module
            MockValue::Str("get".to_string()),              // callee_function
            MockValue::Int(1),                              // callee_arity
            MockValue::Str("lib/my_app/controller.ex".to_string()), // file
            MockValue::Int(25),                             // call_line
        ])
    }

    #[rstest]
    fn test_extract_call_from_row_trait_valid_row() {
        let layout = standard_layout();
        let row = valid_call_row();

        let call = extract_call_from_row_trait(&row, &layout);
        assert!(call.is_some(), "Valid row should produce Some(Call)");

        let call = call.unwrap();
        assert_eq!(call.caller.module.as_ref(), "MyApp.Controller");
        assert_eq!(call.caller.name.as_ref(), "index");
        assert_eq!(call.caller.arity, 2);
        assert_eq!(call.caller.kind.as_deref(), Some("def"));
        assert_eq!(call.caller.start_line, Some(10));
        assert_eq!(call.caller.end_line, Some(30));
        assert_eq!(
            call.caller.file.as_deref(),
            Some("lib/my_app/controller.ex")
        );

        assert_eq!(call.callee.module.as_ref(), "MyApp.Repo");
        assert_eq!(call.callee.name.as_ref(), "get");
        assert_eq!(call.callee.arity, 1);

        assert_eq!(call.line, 25);
        assert!(call.call_type.is_none());
        assert!(call.depth.is_none());
    }

    #[rstest]
    fn test_extract_call_from_row_trait_missing_caller_module_returns_none() {
        let layout = standard_layout();
        // Replace caller_module (index 0) with None to simulate missing required field
        let row = MockRow::new(vec![
            MockValue::None,                                // caller_module - missing
            MockValue::Str("index".to_string()),            // caller_name
            MockValue::Int(2),                              // caller_arity
            MockValue::Str("def".to_string()),              // caller_kind
            MockValue::Int(10),                             // caller_start_line
            MockValue::Int(30),                             // caller_end_line
            MockValue::Str("MyApp.Repo".to_string()),       // callee_module
            MockValue::Str("get".to_string()),              // callee_function
            MockValue::Int(1),                              // callee_arity
            MockValue::Str("lib/controller.ex".to_string()), // file
            MockValue::Int(25),                             // call_line
        ]);

        let call = extract_call_from_row_trait(&row, &layout);
        assert!(call.is_none(), "Missing caller_module should return None");
    }

    #[rstest]
    fn test_extract_call_from_row_trait_missing_callee_name_returns_none() {
        let layout = standard_layout();
        let row = MockRow::new(vec![
            MockValue::Str("MyApp.Controller".to_string()), // caller_module
            MockValue::Str("index".to_string()),            // caller_name
            MockValue::Int(2),                              // caller_arity
            MockValue::Str("def".to_string()),              // caller_kind
            MockValue::Int(10),                             // caller_start_line
            MockValue::Int(30),                             // caller_end_line
            MockValue::Str("MyApp.Repo".to_string()),       // callee_module
            MockValue::None,                                // callee_function - missing
            MockValue::Int(1),                              // callee_arity
            MockValue::Str("lib/controller.ex".to_string()), // file
            MockValue::Int(25),                             // call_line
        ]);

        let call = extract_call_from_row_trait(&row, &layout);
        assert!(
            call.is_none(),
            "Missing callee function name should return None"
        );
    }

    #[rstest]
    fn test_extract_call_from_row_trait_missing_file_returns_none() {
        let layout = standard_layout();
        let row = MockRow::new(vec![
            MockValue::Str("MyApp.Controller".to_string()),
            MockValue::Str("index".to_string()),
            MockValue::Int(2),
            MockValue::Str("def".to_string()),
            MockValue::Int(10),
            MockValue::Int(30),
            MockValue::Str("MyApp.Repo".to_string()),
            MockValue::Str("get".to_string()),
            MockValue::Int(1),
            MockValue::None, // file - missing
            MockValue::Int(25),
        ]);

        let call = extract_call_from_row_trait(&row, &layout);
        assert!(call.is_none(), "Missing file should return None");
    }

    #[rstest]
    fn test_extract_call_from_row_trait_empty_row_returns_none() {
        let layout = standard_layout();
        let row = MockRow::new(vec![]);

        let call = extract_call_from_row_trait(&row, &layout);
        assert!(call.is_none(), "Empty row should return None");
    }

    // CallRowLayout::from_headers tests

    fn standard_headers() -> Vec<String> {
        vec![
            "caller_module",
            "caller_name",
            "caller_arity",
            "caller_kind",
            "caller_start_line",
            "caller_end_line",
            "callee_module",
            "callee_function",
            "callee_arity",
            "file",
            "call_line",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[rstest]
    fn test_from_headers_all_required_columns() {
        let headers = standard_headers();
        let layout = CallRowLayout::from_headers(&headers).unwrap();

        assert_eq!(layout.caller_module_idx, 0);
        assert_eq!(layout.caller_name_idx, 1);
        assert_eq!(layout.caller_arity_idx, 2);
        assert_eq!(layout.caller_kind_idx, 3);
        assert_eq!(layout.caller_start_line_idx, 4);
        assert_eq!(layout.caller_end_line_idx, 5);
        assert_eq!(layout.callee_module_idx, 6);
        assert_eq!(layout.callee_name_idx, 7);
        assert_eq!(layout.callee_arity_idx, 8);
        assert_eq!(layout.file_idx, 9);
        assert_eq!(layout.line_idx, 10);
        assert_eq!(layout.call_type_idx, None);
    }

    #[rstest]
    fn test_from_headers_with_optional_call_type() {
        let mut headers = standard_headers();
        headers.push("call_type".to_string());

        let layout = CallRowLayout::from_headers(&headers).unwrap();
        assert_eq!(layout.call_type_idx, Some(11));
    }

    #[rstest]
    fn test_from_headers_different_column_order() {
        // Columns in different order - the key benefit of dynamic lookup
        let headers: Vec<String> = vec![
            "file",
            "callee_module",
            "caller_module",
            "call_line",
            "caller_name",
            "callee_function",
            "caller_arity",
            "callee_arity",
            "caller_kind",
            "caller_start_line",
            "caller_end_line",
            "call_type",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let layout = CallRowLayout::from_headers(&headers).unwrap();

        assert_eq!(layout.file_idx, 0);
        assert_eq!(layout.callee_module_idx, 1);
        assert_eq!(layout.caller_module_idx, 2);
        assert_eq!(layout.line_idx, 3);
        assert_eq!(layout.caller_name_idx, 4);
        assert_eq!(layout.callee_name_idx, 5);
        assert_eq!(layout.caller_arity_idx, 6);
        assert_eq!(layout.callee_arity_idx, 7);
        assert_eq!(layout.caller_kind_idx, 8);
        assert_eq!(layout.caller_start_line_idx, 9);
        assert_eq!(layout.caller_end_line_idx, 10);
        assert_eq!(layout.call_type_idx, Some(11));
    }

    #[rstest]
    fn test_from_headers_missing_required_column() {
        let headers: Vec<String> = vec![
            "caller_module",
            "caller_name",
            // missing caller_arity
            "caller_kind",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = CallRowLayout::from_headers(&headers);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, DbError::MissingColumn { name } if name == "caller_arity"));
    }

    #[rstest]
    fn test_from_headers_error_message() {
        let headers: Vec<String> = vec!["caller_module".to_string()];

        let err = CallRowLayout::from_headers(&headers).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Missing column 'caller_name' in query result"
        );
    }

    // try_create_relation tests

    #[rstest]
    fn test_try_create_relation_success_when_created() {
        let db = open_mem_db().expect("Failed to create in-memory DB");

        // Create a simple test table - should succeed and return Ok(true)
        let script = r#"DEFINE TABLE test_relation SCHEMAFULL"#;
        let result = try_create_relation(&*db, script);
        assert!(
            result.is_ok(),
            "Creation of new relation should succeed: {:?}",
            result
        );
        assert_eq!(result.unwrap(), true);
    }

    #[rstest]
    fn test_try_create_relation_idempotent_on_second_call() {
        let db = open_mem_db().expect("Failed to create in-memory DB");

        // Create a test table first time
        let script = r#"DEFINE TABLE test_relation_idempotent SCHEMAFULL"#;
        let result1 = try_create_relation(&*db, script);
        assert!(result1.is_ok(), "First creation should succeed");
        assert_eq!(result1.unwrap(), true);

        // Try to create the same table again - should detect it exists
        let result2 = try_create_relation(&*db, script);
        assert!(result2.is_ok(), "Second creation attempt should not error");
        assert_eq!(
            result2.unwrap(),
            false,
            "Second call should report already exists"
        );
    }

    #[rstest]
    fn test_try_create_relation_propagates_genuine_errors() {
        let db = open_mem_db().expect("Failed to create in-memory DB");

        // Invalid SurrealQL that will cause a real error (not "already exists")
        let invalid_script = "invalid syntax here !!!";
        let result = try_create_relation(&*db, invalid_script);
        assert!(result.is_err(), "Should propagate genuine syntax errors");
    }
}
