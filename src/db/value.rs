//! Trait abstraction for database values.
//!
//! This module defines the `DatabaseValue` trait that abstracts different database
//! value types (e.g., `cozo::DataValue`, future PostgreSQL types). This enables
//! backend-agnostic result processing.

#![allow(dead_code)] // Some trait methods used by future backends after Ticket #44

use std::fmt::Debug;
use cozo::{DataValue, Num};

/// Trait for database values that can be extracted to Rust types.
///
/// This trait abstracts the value representation of different database backends,
/// allowing query results to be processed uniformly regardless of the underlying
/// database system.
pub trait DatabaseValue: Clone + Debug {
    /// Extract as String if the value is string-like.
    ///
    /// Returns `Some(String)` if the value can be represented as a string,
    /// `None` if the value is null or not string-like.
    fn as_string(&self) -> Option<String>;

    /// Extract as i64 if the value is numeric.
    ///
    /// Returns `Some(i64)` if the value is an integer or can be converted to one
    /// (e.g., float truncated to int), `None` otherwise.
    fn as_i64(&self) -> Option<i64>;

    /// Extract as f64 if the value is a float.
    ///
    /// Returns `Some(f64)` if the value is a floating-point number or integer,
    /// `None` otherwise.
    fn as_f64(&self) -> Option<f64>;

    /// Extract as bool if the value is boolean.
    ///
    /// Returns `Some(bool)` if the value is a boolean, `None` otherwise.
    fn as_bool(&self) -> Option<bool>;

    /// Get type name for debugging/error messages.
    ///
    /// Returns a static string describing the value's type.
    /// Will be used by future backends for error messages and introspection.
    fn type_name(&self) -> &'static str;

    /// Extract as i64 with a default value.
    ///
    /// Returns the extracted i64 or the provided default if extraction fails.
    fn as_i64_or(&self, default: i64) -> i64 {
        self.as_i64().unwrap_or(default)
    }

    /// Extract as String with a default value.
    ///
    /// Returns the extracted String or the provided default if extraction fails.
    fn as_string_or(&self, default: &str) -> String {
        self.as_string().unwrap_or_else(|| default.to_string())
    }

    /// Extract as bool with a default value.
    ///
    /// Returns the extracted bool or the provided default if extraction fails.
    fn as_bool_or(&self, default: bool) -> bool {
        self.as_bool().unwrap_or(default)
    }
}

/// Implementation of `DatabaseValue` for `cozo::DataValue`.
///
/// Provides extraction methods for all CozoDB value types.
impl DatabaseValue for DataValue {
    fn as_string(&self) -> Option<String> {
        match self {
            DataValue::Str(s) => Some(s.to_string()),
            DataValue::Null => None,
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            DataValue::Num(Num::Int(i)) => Some(*i),
            DataValue::Num(Num::Float(f)) => Some(*f as i64),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            DataValue::Num(Num::Int(i)) => Some(*i as f64),
            DataValue::Num(Num::Float(f)) => Some(*f),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            DataValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            DataValue::Null => "null",
            DataValue::Bool(_) => "bool",
            DataValue::Num(_) => "number",
            DataValue::Str(_) => "string",
            DataValue::Bytes(_) => "bytes",
            DataValue::List(_) => "list",
            DataValue::Set(_) => "set",
            DataValue::Vec(_) => "vec",
            DataValue::Json(_) => "json",
            DataValue::Uuid(_) => "uuid",
            DataValue::Regex(_) => "regex",
            DataValue::Bot => "bot",
            DataValue::Validity(_) => "validity",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock DatabaseValue implementation for testing trait behavior
    #[derive(Clone, Debug)]
    struct MockValue {
        val: String,
    }

    impl MockValue {
        fn new(val: &str) -> Self {
            Self {
                val: val.to_string(),
            }
        }
    }

    impl DatabaseValue for MockValue {
        fn as_string(&self) -> Option<String> {
            Some(self.val.clone())
        }

        fn as_i64(&self) -> Option<i64> {
            self.val.parse().ok()
        }

        fn as_f64(&self) -> Option<f64> {
            self.val.parse().ok()
        }

        fn as_bool(&self) -> Option<bool> {
            match self.val.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        }

        fn type_name(&self) -> &'static str {
            "mock"
        }
    }

    #[test]
    fn test_as_string_or_default() {
        let val = MockValue::new("hello");
        assert_eq!(val.as_string_or("default"), "hello");
    }

    #[test]
    fn test_as_i64_or_default() {
        let val = MockValue::new("42");
        assert_eq!(val.as_i64_or(0), 42);
    }

    #[test]
    fn test_as_bool_or_default() {
        let val = MockValue::new("true");
        assert_eq!(val.as_bool_or(false), true);
    }

    // Tests for DataValue implementation
    #[test]
    fn test_datavalue_as_string() {
        let val = DataValue::Str("hello".into());
        assert_eq!(val.as_string(), Some("hello".to_string()));
    }

    #[test]
    fn test_datavalue_as_string_null() {
        let val = DataValue::Null;
        assert_eq!(val.as_string(), None);
    }

    #[test]
    fn test_datavalue_as_string_non_string() {
        let val = DataValue::Num(Num::Int(42));
        assert_eq!(val.as_string(), None);
    }

    #[test]
    fn test_datavalue_as_i64_from_int() {
        let val = DataValue::Num(Num::Int(42));
        assert_eq!(val.as_i64(), Some(42));
    }

    #[test]
    fn test_datavalue_as_i64_from_float() {
        let val = DataValue::Num(Num::Float(42.7));
        assert_eq!(val.as_i64(), Some(42));
    }

    #[test]
    fn test_datavalue_as_i64_non_numeric() {
        let val = DataValue::Str("not a number".into());
        assert_eq!(val.as_i64(), None);
    }

    #[test]
    fn test_datavalue_as_f64_from_int() {
        let val = DataValue::Num(Num::Int(42));
        assert_eq!(val.as_f64(), Some(42.0));
    }

    #[test]
    fn test_datavalue_as_f64_from_float() {
        let val = DataValue::Num(Num::Float(42.7));
        assert_eq!(val.as_f64(), Some(42.7));
    }

    #[test]
    fn test_datavalue_as_bool_true() {
        let val = DataValue::Bool(true);
        assert_eq!(val.as_bool(), Some(true));
    }

    #[test]
    fn test_datavalue_as_bool_false() {
        let val = DataValue::Bool(false);
        assert_eq!(val.as_bool(), Some(false));
    }

    #[test]
    fn test_datavalue_as_bool_non_bool() {
        let val = DataValue::Str("true".into());
        assert_eq!(val.as_bool(), None);
    }

    #[test]
    fn test_datavalue_type_name_null() {
        assert_eq!(DataValue::Null.type_name(), "null");
    }

    #[test]
    fn test_datavalue_type_name_bool() {
        assert_eq!(DataValue::Bool(true).type_name(), "bool");
    }

    #[test]
    fn test_datavalue_type_name_num() {
        assert_eq!(DataValue::Num(Num::Int(0)).type_name(), "number");
    }

    #[test]
    fn test_datavalue_type_name_str() {
        assert_eq!(DataValue::Str("test".into()).type_name(), "string");
    }

    #[test]
    fn test_datavalue_as_i64_or_with_value() {
        let val = DataValue::Num(Num::Int(42));
        assert_eq!(val.as_i64_or(0), 42);
    }

    #[test]
    fn test_datavalue_as_i64_or_with_default() {
        let val = DataValue::Str("not a number".into());
        assert_eq!(val.as_i64_or(99), 99);
    }

    #[test]
    fn test_datavalue_as_string_or_with_value() {
        let val = DataValue::Str("hello".into());
        assert_eq!(val.as_string_or("default"), "hello");
    }

    #[test]
    fn test_datavalue_as_string_or_with_default() {
        let val = DataValue::Num(Num::Int(42));
        assert_eq!(val.as_string_or("default"), "default");
    }

    #[test]
    fn test_datavalue_as_bool_or_with_value() {
        let val = DataValue::Bool(true);
        assert_eq!(val.as_bool_or(false), true);
    }

    #[test]
    fn test_datavalue_as_bool_or_with_default() {
        let val = DataValue::Str("not a bool".into());
        assert_eq!(val.as_bool_or(false), false);
    }
}
