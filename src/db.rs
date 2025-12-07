use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use cozo::{DataValue, DbInstance, NamedRows, ScriptMutability};
use thiserror::Error;

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

/// Escape a string for use in CozoDB string literals
pub fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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
