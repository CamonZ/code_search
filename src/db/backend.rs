//! Database backend trait for abstracting different database implementations.
//!
//! Phase 1 infrastructure (Ticket #41). The trait methods will be used when
//! Ticket #44 updates the Execute trait to accept `&dyn DatabaseBackend`
//! instead of `&DbInstance`.

use std::error::Error;
use cozo::DataValue;
use super::value::DatabaseValue;
use super::schema::SchemaRelation;

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

    /// Insert rows into a relation/table.
    ///
    /// Takes a schema relation definition and rows as vectors of DataValues.
    /// Returns the number of rows inserted.
    ///
    /// # Arguments
    /// * `relation` - The schema relation to insert into
    /// * `rows` - Vector of rows, each row is a Vec<DataValue> matching the relation's fields
    ///
    /// # Errors
    /// Returns an error if the insert fails (e.g., constraint violation, connection error)
    fn insert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>>;

    /// Delete rows matching a project filter.
    ///
    /// Deletes all rows where the `project` field matches the given value.
    /// This is the primary deletion pattern used during import (clear before reimport).
    ///
    /// # Arguments
    /// * `relation` - The schema relation to delete from
    /// * `project` - The project identifier to match
    ///
    /// # Returns
    /// Number of rows deleted (0 if backend doesn't support delete counts)
    fn delete_by_project(
        &self,
        relation: &SchemaRelation,
        project: &str,
    ) -> Result<usize, Box<dyn Error>>;

    /// Upsert rows (insert or update on conflict).
    ///
    /// For backends that support upsert semantics, this inserts new rows
    /// or updates existing rows if the key already exists.
    ///
    /// # Default Implementation
    /// Falls back to `insert_rows()` for backends where insert is already an upsert (like Cozo).
    fn upsert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        self.insert_rows(relation, rows)
    }

    /// Get the underlying DbInstance for use with existing query functions.
    /// This provides a migration path during the transition to the trait-based API.
    fn as_db_instance(&self) -> &cozo::DbInstance;

    /// Perform backend-specific setup/initialization.
    ///
    /// This is called by the `setup` command before `run_migrations()` to
    /// perform any backend-specific initialization that needs to happen
    /// before schema creation.
    ///
    /// # Default Implementation
    /// Returns Ok(()) for backends that don't need special initialization.
    ///
    /// # PostgreSQL AGE
    /// Creates the AGE graph if it doesn't exist.
    fn setup_backend(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
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

    #[test]
    fn test_trait_has_insert_rows_method() {
        // Verify the trait can be used as a trait object with the new method
        // This is a compile-time check that the trait is object-safe
        fn accepts_backend(_db: &dyn DatabaseBackend) {}
        let _ = accepts_backend;
    }

    #[test]
    fn test_upsert_default_implementation() {
        // The default upsert should call insert_rows
        // This is tested via the concrete implementations
        // At this level, we just verify the trait method exists and is callable
        fn uses_upsert(_db: &dyn DatabaseBackend) {}
        let _ = uses_upsert;
    }
}
