//! Backend-specific query compilation helpers.
//!
//! Provides an abstraction layer for backend-specific syntax generation,
//! allowing query builders to compile to multiple backend syntaxes
//! (Cozo Datalog, AGE Cypher, etc.).

pub mod cozo;
pub mod age;

use crate::db::DatabaseBackend;

/// Trait for backend-specific query compilation.
///
/// Different database backends require different syntax for placeholders,
/// string escaping, and other query elements. This trait encapsulates
/// those differences.
pub trait BackendCompiler {
    /// Get the parameter placeholder for a given parameter name.
    ///
    /// For example:
    /// - Cozo: `parameter_placeholder("limit")` -> `"$limit"`
    /// - AGE: `parameter_placeholder("limit")` -> `"$limit"`
    /// - SQL: `parameter_placeholder("limit")` -> `"?"` or `":limit"`
    fn parameter_placeholder(&self, name: &str) -> String;

    /// Escape a string literal for use in a query.
    ///
    /// Handles backend-specific escaping rules (quote characters, special chars, etc.)
    fn escape_string(&self, s: &str) -> String;

    /// Compile a filter expression (field, operator, parameter).
    ///
    /// Combines a field name, comparison operator, and parameter into
    /// a backend-specific filter expression.
    fn compile_filter(&self, field: &str, op: &str, param: &str) -> String;
}

/// Get the appropriate compiler for a database backend.
pub fn get_compiler(backend: &dyn DatabaseBackend) -> Box<dyn BackendCompiler> {
    match backend.backend_name() {
        "CozoSqlite" | "CozoRocksdb" | "CozoMem" => Box::new(cozo::CozoCompiler),
        "PostgresAge" => Box::new(age::AgeCompiler),
        _ => panic!("Unsupported backend: {}", backend.backend_name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_get_compiler_cozo() {
        let backend = open_mem_db().unwrap();
        let _compiler = get_compiler(backend.as_ref());
        // If we got here without panicking, the test passed
    }

    #[test]
    fn test_backend_compiler_trait_object() {
        let backend = open_mem_db().unwrap();
        let compiler = get_compiler(backend.as_ref());

        // Test that we can call trait methods
        let placeholder = compiler.parameter_placeholder("test");
        assert_eq!(placeholder, "$test");
    }
}
