//! Database connection and query utilities for CozoDB.
//!
//! This module provides the database abstraction layer for the CLI tool:
//! - Connection management (SQLite-backed or in-memory for tests)
//! - Query execution with parameter binding
//! - Result row extraction with type-safe helpers
//!
//! # Architecture
//!
//! CozoDB is a Datalog database that stores call graph data in relations.
//! Queries are written in CozoScript (a Datalog variant) and return `NamedRows`
//! containing `DataValue` cells that must be extracted into Rust types.
//!
//! # Type Decisions
//!
//! **Why `i64` for arity/line numbers instead of `u32`?**
//! CozoDB returns all integers as `Num::Int(i64)`. Using `i64` throughout avoids
//! lossy conversions and potential panics. The semantic constraint (arity >= 0)
//! is enforced by the data source (Elixir AST), not runtime checks.
//!
//! **Why `CallRowLayout` with indices instead of serde deserialization?**
//! CozoDB returns rows as `Vec<DataValue>`, not JSON objects. The `CallRowLayout`
//! struct documents column positions for each query type, centralizing the
//! mapping in two factory methods rather than scattering magic numbers.
//!
//! **Why bare `String` for module/function names instead of newtypes?**
//! For a CLI tool, the complexity of newtype wrappers (`.0` access, `Into` impls,
//! derive macro limitations) outweighs the type safety benefit. Field names
//! (`module`, `name`) are sufficiently clear.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use cozo::{DataValue, DbInstance, NamedRows, ScriptMutability};
use thiserror::Error;

use crate::types::{Call, FunctionRef};

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Failed to open database '{path}': {message}")]
    OpenFailed { path: String, message: String },

    #[error("Query failed: {message}")]
    QueryFailed { message: String },
}

pub type Params = BTreeMap<String, DataValue>;

pub fn open_db(path: &Path) -> Result<DbInstance, Box<dyn Error>> {
    DbInstance::new("sqlite", path, "").map_err(|e| {
        Box::new(DbError::OpenFailed {
            path: path.display().to_string(),
            message: format!("{:?}", e),
        }) as Box<dyn Error>
    })
}

/// Create an in-memory database instance.
///
/// Used for tests to avoid disk I/O and temp file management.
#[cfg(test)]
pub fn open_mem_db() -> DbInstance {
    DbInstance::new("mem", "", "").expect("Failed to create in-memory DB")
}

/// Run a mutable query (insert, delete, create, etc.)
pub fn run_query(
    db: &DbInstance,
    script: &str,
    params: Params,
) -> Result<NamedRows, Box<dyn Error>> {
    db.run_script(script, params, ScriptMutability::Mutable)
        .map_err(|e| {
            Box::new(DbError::QueryFailed {
                message: format!("{:?}", e),
            }) as Box<dyn Error>
        })
}

/// Run a mutable query with no parameters
pub fn run_query_no_params(db: &DbInstance, script: &str) -> Result<NamedRows, Box<dyn Error>> {
    run_query(db, script, Params::new())
}

/// Escape a string for use in CozoDB double-quoted string literals (JSON-compatible)
pub fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
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

/// Escape a string for use in CozoDB single-quoted string literals.
/// Use this for strings that may contain double quotes or complex content.
pub fn escape_string_single(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '\'' => result.push_str("\\'"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() || c == '\0' => {
                // Escape control characters as \uXXXX
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// Try to create a relation, returning Ok(true) if created, Ok(false) if already exists
pub fn try_create_relation(db: &DbInstance, script: &str) -> Result<bool, Box<dyn Error>> {
    match run_query_no_params(db, script) {
        Ok(_) => Ok(true),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("AlreadyExists") || err_str.contains("stored_relation_conflict") {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

// DataValue extraction helpers

use cozo::Num;

/// Extract a String from a DataValue, returning None if not a string
pub fn extract_string(value: &DataValue) -> Option<String> {
    match value {
        DataValue::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

/// Extract an i64 from a DataValue, returning the default if not a number
pub fn extract_i64(value: &DataValue, default: i64) -> i64 {
    match value {
        DataValue::Num(Num::Int(i)) => *i,
        DataValue::Num(Num::Float(f)) => *f as i64,
        _ => default,
    }
}

/// Extract a String from a DataValue, returning the default if not a string
pub fn extract_string_or(value: &DataValue, default: &str) -> String {
    match value {
        DataValue::Str(s) => s.to_string(),
        _ => default.to_string(),
    }
}

/// Extract a bool from a DataValue, returning the default if not a bool
pub fn extract_bool(value: &DataValue, default: bool) -> bool {
    match value {
        DataValue::Bool(b) => *b,
        _ => default,
    }
}

/// Layout descriptor for extracting call data from query result rows
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
    /// Standard layout for queries that include project at [0] and call_type at [12]
    /// Used by: calls_from, calls_to
    pub fn with_project_and_type() -> Self {
        Self {
            caller_module_idx: 1,
            caller_name_idx: 2,
            caller_arity_idx: 3,
            caller_kind_idx: 4,
            caller_start_line_idx: 5,
            caller_end_line_idx: 6,
            callee_module_idx: 7,
            callee_name_idx: 8,
            callee_arity_idx: 9,
            file_idx: 10,
            line_idx: 11,
            call_type_idx: Some(12),
        }
    }

    /// Standard layout for queries without project or call_type
    /// Used by: depends_on, depended_by
    pub fn without_extras() -> Self {
        Self {
            caller_module_idx: 0,
            caller_name_idx: 1,
            caller_arity_idx: 2,
            caller_kind_idx: 3,
            caller_start_line_idx: 4,
            caller_end_line_idx: 5,
            callee_module_idx: 6,
            callee_name_idx: 7,
            callee_arity_idx: 8,
            file_idx: 9,
            line_idx: 10,
            call_type_idx: None,
        }
    }
}

/// Extract call data from a query result row
///
/// Returns Option<Call> if all required fields are present. Uses early return
/// (None) if any required string field cannot be extracted.
pub fn extract_call_from_row(row: &[DataValue], layout: &CallRowLayout) -> Option<Call> {
    // Extract caller information
    let Some(caller_module) = extract_string(&row[layout.caller_module_idx]) else { return None };
    let Some(caller_name) = extract_string(&row[layout.caller_name_idx]) else { return None };
    let caller_arity = extract_i64(&row[layout.caller_arity_idx], 0);
    let caller_kind = extract_string_or(&row[layout.caller_kind_idx], "");
    let caller_start_line = extract_i64(&row[layout.caller_start_line_idx], 0);
    let caller_end_line = extract_i64(&row[layout.caller_end_line_idx], 0);

    // Extract callee information
    let Some(callee_module) = extract_string(&row[layout.callee_module_idx]) else { return None };
    let Some(callee_name) = extract_string(&row[layout.callee_name_idx]) else { return None };
    let callee_arity = extract_i64(&row[layout.callee_arity_idx], 0);

    // Extract file and line
    let Some(file) = extract_string(&row[layout.file_idx]) else { return None };
    let line = extract_i64(&row[layout.line_idx], 0);

    // Extract optional call_type
    let call_type = layout.call_type_idx.and_then(|idx| {
        if idx < row.len() {
            Some(extract_string_or(&row[idx], "remote"))
        } else {
            None
        }
    });

    // Create FunctionRef objects
    let caller = FunctionRef::with_definition(
        caller_module,
        caller_name,
        caller_arity,
        caller_kind,
        &file,
        caller_start_line,
        caller_end_line,
    );

    let callee = FunctionRef::new(callee_module, callee_name, callee_arity);

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
    use cozo::Num;
    use rstest::rstest;

    #[rstest]
    fn test_extract_string_from_str() {
        let value = DataValue::Str("hello".into());
        assert_eq!(extract_string(&value), Some("hello".to_string()));
    }

    #[rstest]
    fn test_extract_string_from_non_str() {
        let value = DataValue::Num(Num::Int(42));
        assert_eq!(extract_string(&value), None);
    }

    #[rstest]
    fn test_extract_i64_from_int() {
        let value = DataValue::Num(Num::Int(42));
        assert_eq!(extract_i64(&value, 0), 42);
    }

    #[rstest]
    fn test_extract_i64_from_float() {
        let value = DataValue::Num(Num::Float(42.7));
        assert_eq!(extract_i64(&value, 0), 42);
    }

    #[rstest]
    fn test_extract_i64_from_non_num() {
        let value = DataValue::Str("not a number".into());
        assert_eq!(extract_i64(&value, -1), -1);
    }

    #[rstest]
    fn test_extract_string_or_from_str() {
        let value = DataValue::Str("hello".into());
        assert_eq!(extract_string_or(&value, "default"), "hello");
    }

    #[rstest]
    fn test_extract_string_or_from_non_str() {
        let value = DataValue::Num(Num::Int(42));
        assert_eq!(extract_string_or(&value, "default"), "default");
    }

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

    #[rstest]
    fn test_extract_bool_from_bool() {
        let value = DataValue::Bool(true);
        assert_eq!(extract_bool(&value, false), true);
    }

    #[rstest]
    fn test_extract_bool_from_non_bool() {
        let value = DataValue::Str("true".into());
        assert_eq!(extract_bool(&value, false), false);
    }
}
