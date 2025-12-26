//! Backend abstraction layer for database operations.
//!
//! This module provides trait definitions that abstract database operations,
//! allowing both CozoDB and SurrealDB backends to implement the same interface.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

/// Backend-agnostic parameter types for database queries.
///
/// Variants represent the different types of values that can be passed
/// as parameters to database queries.
#[derive(Clone, Debug)]
pub enum ValueType {
    /// String value
    Str(String),
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Bool(bool),
}

/// Container for query parameters.
///
/// Maps parameter names to their values, allowing type-safe parameter
/// binding for database queries across different backend implementations.
#[derive(Debug, Default)]
pub struct QueryParams {
    params: BTreeMap<String, ValueType>,
}

impl QueryParams {
    /// Creates a new empty parameter container.
    pub fn new() -> Self {
        Self {
            params: BTreeMap::new(),
        }
    }

    /// Inserts a parameter with a string value.
    pub fn with_str(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), ValueType::Str(value.into()));
        self
    }

    /// Inserts a parameter with an integer value.
    pub fn with_int(mut self, key: impl Into<String>, value: i64) -> Self {
        self.params.insert(key.into(), ValueType::Int(value));
        self
    }

    /// Inserts a parameter with a float value.
    pub fn with_float(mut self, key: impl Into<String>, value: f64) -> Self {
        self.params.insert(key.into(), ValueType::Float(value));
        self
    }

    /// Inserts a parameter with a boolean value.
    pub fn with_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.params.insert(key.into(), ValueType::Bool(value));
        self
    }

    /// Returns a reference to the underlying parameters map.
    pub fn params(&self) -> &BTreeMap<String, ValueType> {
        &self.params
    }
}

/// Trait for extracting typed values from database rows.
///
/// Implementations should provide type conversion methods that safely
/// extract values from the underlying database representation.
pub trait Value: Send + Sync + std::fmt::Debug {
    /// Attempts to extract the value as a string reference.
    fn as_str(&self) -> Option<&str>;

    /// Attempts to extract the value as a signed 64-bit integer.
    fn as_i64(&self) -> Option<i64>;

    /// Attempts to extract the value as a 64-bit float.
    fn as_f64(&self) -> Option<f64>;

    /// Attempts to extract the value as a boolean.
    fn as_bool(&self) -> Option<bool>;

    /// Attempts to extract the value as an array of values.
    fn as_array(&self) -> Option<Vec<&dyn Value>>;

    /// Attempts to extract the id from a SurrealDB Thing (record reference).
    /// Returns the id as a Value which can be further extracted (e.g., as an array).
    fn as_thing_id(&self) -> Option<&dyn Value>;
}

/// Trait for accessing column values in a database row.
///
/// A row represents a single result row from a query, providing access
/// to individual column values by index.
pub trait Row: Send + Sync {
    /// Retrieves the value at the specified column index.
    fn get(&self, index: usize) -> Option<&dyn Value>;

    /// Returns the number of columns in this row.
    fn len(&self) -> usize;

    /// Returns true if the row is empty (contains no columns).
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Trait for accessing results from a database query.
///
/// A query result contains headers (column names) and rows of data,
/// providing both immutable and owned access to the result set.
pub trait QueryResult: Send + Sync {
    /// Returns the names of columns in the result set.
    fn headers(&self) -> &[String];

    /// Returns references to the rows in the result set.
    fn rows(&self) -> &[Box<dyn Row>];

    /// Consumes this result and returns the rows as an owned vector.
    fn into_rows(self: Box<Self>) -> Vec<Box<dyn Row>>;
}

/// Core trait for database operations.
///
/// Implementations should handle query execution and parameter binding,
/// returning results in a backend-agnostic format. All implementations
/// must be thread-safe (Send + Sync).
pub trait Database: Send + Sync {
    /// Executes a query with the provided parameters.
    fn execute_query(
        &self,
        query: &str,
        params: QueryParams,
    ) -> Result<Box<dyn QueryResult>, Box<dyn Error>>;

    /// Executes a query without parameters.
    ///
    /// This is a convenience method that calls `execute_query` with
    /// empty parameters.
    fn execute_query_no_params(&self, query: &str) -> Result<Box<dyn QueryResult>, Box<dyn Error>> {
        self.execute_query(query, QueryParams::new())
    }

    /// Returns the underlying database instance as a trait object.
    ///
    /// Used for testing and downcasting in backend-specific code.
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync);
}

#[cfg(feature = "backend-cozo")]
pub(crate) mod cozo;
#[cfg(feature = "backend-cozo")]
pub mod cozo_schema;

#[cfg(feature = "backend-surrealdb")]
pub(crate) mod surrealdb;
#[cfg(feature = "backend-surrealdb")]
pub mod surrealdb_schema;

/// Opens a database connection to the specified path.
///
/// This function uses feature flags to determine which backend to use:
/// - `backend-cozo`: Opens a CozoDB instance
/// - `backend-surrealdb`: Opens a SurrealDB instance
///
/// At least one backend feature must be enabled.
#[cfg(all(feature = "backend-cozo", not(feature = "backend-surrealdb")))]
pub fn open_database(path: &Path) -> Result<Box<dyn Database>, Box<dyn Error>> {
    Ok(Box::new(cozo::CozoDatabase::open(path)?))
}

#[cfg(all(feature = "backend-surrealdb", not(feature = "backend-cozo")))]
pub fn open_database(path: &Path) -> Result<Box<dyn Database>, Box<dyn Error>> {
    Ok(Box::new(surrealdb::SurrealDatabase::open(path)?))
}

#[cfg(all(feature = "backend-cozo", feature = "backend-surrealdb"))]
compile_error!("Cannot enable both backend-cozo and backend-surrealdb features at the same time");

#[cfg(not(any(feature = "backend-cozo", feature = "backend-surrealdb")))]
pub fn open_database(_path: &Path) -> Result<Box<dyn Database>, Box<dyn Error>> {
    compile_error!("Must enable either backend-cozo or backend-surrealdb")
}

/// Opens an in-memory database for testing.
///
/// This function is only available when building tests or when the
/// `test-utils` feature is enabled.
///
/// This should use the default backend (determined by feature flags)
/// in in-memory mode.
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-cozo", not(feature = "backend-surrealdb")))]
pub fn open_mem_database() -> Result<Box<dyn Database>, Box<dyn Error>> {
    Ok(Box::new(cozo::CozoDatabase::open_mem()))
}

#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb", not(feature = "backend-cozo")))]
pub fn open_mem_database() -> Result<Box<dyn Database>, Box<dyn Error>> {
    Ok(Box::new(surrealdb::SurrealDatabase::open_mem()?))
}

#[cfg(all(any(test, feature = "test-utils"), not(any(feature = "backend-cozo", feature = "backend-surrealdb"))))]
pub fn open_mem_database() -> Result<Box<dyn Database>, Box<dyn Error>> {
    compile_error!("Must enable either backend-cozo or backend-surrealdb")
}
