//! Database backend trait for abstracting different database implementations.
//!
//! Phase 1 infrastructure (Ticket #41). The trait methods will be used when
//! Ticket #44 updates the Execute trait to accept `&dyn DatabaseBackend`
//! instead of `&DbInstance`.

use std::error::Error;
use cozo::DataValue;
use super::value::DatabaseValue;

/// Result of a query execution.
///
/// Generic over value type to support different database backends.
/// Defaults to `cozo::DataValue` for the CozoDB backend.
///
/// Currently used only in tests; will be the return type for Execute trait
/// after Ticket #44.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields read after Ticket #44
pub struct QueryResult<V: DatabaseValue = DataValue> {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<V>>,
}

/// Trait for database backends that can execute queries.
///
/// Currently only `as_db_instance()` is used in production code. Other methods
/// are tested and will be used after Ticket #44 migrates commands to the
/// trait-based API.
#[allow(unused)]
pub trait DatabaseBackend: Send + Sync {
    /// Execute a query with parameters, returning raw rows.
    fn execute_query(
        &self,
        script: &str,
        params: &Params,
    ) -> Result<QueryResult<DataValue>, Box<dyn Error>>;

    /// Execute a query without parameters.
    fn execute_query_no_params(
        &self,
        script: &str,
    ) -> Result<QueryResult<DataValue>, Box<dyn Error>> {
        self.execute_query(script, &Params::new())
    }

    /// Get the backend name for logging/debugging.
    fn backend_name(&self) -> &'static str;

    /// Check if a relation (table) exists.
    fn relation_exists(&self, name: &str) -> Result<bool, Box<dyn Error>>;

    /// Create a relation if it doesn't exist.
    /// Returns true if created, false if already existed.
    fn try_create_relation(&self, schema: &str) -> Result<bool, Box<dyn Error>>;

    /// Get the underlying DbInstance for use with existing query functions.
    /// This provides a migration path during the transition to the trait-based API.
    fn as_db_instance(&self) -> &cozo::DbInstance;
}

/// Type alias for query parameters.
pub type Params = std::collections::BTreeMap<String, DataValue>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_result_creation() {
        let result: QueryResult<DataValue> = QueryResult {
            headers: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![vec![DataValue::Num(cozo::Num::Int(1)), DataValue::Str("test".into())]],
        };

        assert_eq!(result.headers.len(), 2);
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_params_creation() {
        let params = Params::new();
        assert_eq!(params.len(), 0);
    }
}
