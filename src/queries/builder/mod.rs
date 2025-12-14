//! Backend-agnostic query building infrastructure.
//!
//! This module provides traits and helpers for defining queries that can compile
//! to multiple database backends (Cozo, PostgreSQL with AGE, etc.).
//!
//! # Architecture
//!
//! The query building system has three layers:
//!
//! 1. **Query Definition** - `QueryBuilder` trait defines what a query is
//! 2. **Compilation** - `BackendCompiler` trait handles backend-specific syntax
//! 3. **Execution** - `CompiledQuery` + database backend execute the compiled query
//!
//! # Example
//!
//! ```ignore
//! let query = SelectQuery {
//!     relation: "functions",
//!     fields: vec!["module", "name"],
//!     filters: vec![],
//!     limit: Some(10),
//! };
//!
//! let compiled = CompiledQuery::from_builder(&query, backend)?;
//! let result = backend.execute_query(&compiled.script, &compiled.params)?;
//! ```

pub mod params;
pub mod compilers;
pub mod patterns;
pub mod helpers;

use std::error::Error;
use crate::db::{DatabaseBackend, Params};

/// Backend-agnostic query definition.
///
/// Implementations of this trait can compile to different database backends
/// and provide their query parameters.
pub trait QueryBuilder: Send + Sync {
    /// Compile this query to a backend-specific query string.
    ///
    /// The returned string is a valid query script for the given backend
    /// (Cozo Datalog or AGE Cypher).
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>>;

    /// Get query parameters (name -> value pairs).
    ///
    /// These parameters are bound into the compiled query using backend-specific
    /// placeholders (e.g., `$name` for Cozo, `$name` for AGE).
    fn parameters(&self) -> Params;

    /// Get the number of parameters.
    fn param_count(&self) -> usize {
        self.parameters().len()
    }
}

/// A compiled query ready for execution.
///
/// Contains the backend-specific query script and all parameters needed
/// for execution.
#[derive(Debug, Clone)]
pub struct CompiledQuery {
    pub script: String,
    pub params: Params,
}

impl CompiledQuery {
    /// Create a compiled query from a builder and backend.
    pub fn from_builder(
        builder: &dyn QueryBuilder,
        backend: &dyn DatabaseBackend,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(CompiledQuery {
            script: builder.compile(backend)?,
            params: builder.parameters(),
        })
    }

    /// Get the number of parameters in this query.
    pub fn param_count(&self) -> usize {
        self.params.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    /// Minimal test QueryBuilder for testing
    struct TestQuery {
        script: String,
        params: Params,
    }

    impl QueryBuilder for TestQuery {
        fn compile(&self, _backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
            Ok(self.script.clone())
        }

        fn parameters(&self) -> Params {
            self.params.clone()
        }
    }

    #[test]
    fn test_compiled_query_from_builder() {
        let backend = open_mem_db().unwrap();
        let params = Params::new();
        let test_query = TestQuery {
            script: "test script".to_string(),
            params,
        };

        let compiled = CompiledQuery::from_builder(&test_query, backend.as_ref()).unwrap();
        assert_eq!(compiled.script, "test script");
        assert_eq!(compiled.param_count(), 0);
    }

    #[test]
    fn test_param_count() {
        let mut params = Params::new();
        params.insert("x".to_string(), cozo::DataValue::Num(cozo::Num::Int(1)));
        params.insert("y".to_string(), cozo::DataValue::Num(cozo::Num::Int(2)));

        let test_query = TestQuery {
            script: "test".to_string(),
            params,
        };

        assert_eq!(test_query.param_count(), 2);
    }

    #[test]
    fn test_compiled_query_clone() {
        let mut params = Params::new();
        params.insert("x".to_string(), cozo::DataValue::Num(cozo::Num::Int(42)));

        let compiled = CompiledQuery {
            script: "SELECT $x".to_string(),
            params,
        };

        let cloned = compiled.clone();
        assert_eq!(cloned.script, compiled.script);
        assert_eq!(cloned.param_count(), compiled.param_count());
    }
}
