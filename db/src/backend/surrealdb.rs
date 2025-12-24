//! SurrealDB backend implementation.
//!
//! This module provides the SurrealDB-specific implementation of the Database trait,
//! wrapping the async SurrealDB API with a synchronous interface using tokio::Runtime.

use super::{Database, QueryParams, QueryResult, Row, Value, ValueType};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
#[allow(unused_imports)]
use surrealdb::engine::local::{Db, RocksDb, Mem};
use surrealdb::Surreal;
use tokio::runtime::Runtime;

/// SurrealDB database wrapper implementing the generic Database trait.
///
/// Uses `tokio::Runtime` to bridge between the async SurrealDB API and the
/// synchronous `Database` trait. The runtime is stored in the struct and used
/// to execute async operations synchronously via `block_on()`.
pub struct SurrealDatabase {
    db: Surreal<Db>,
    runtime: Runtime,
}

impl SurrealDatabase {
    /// Opens a SurrealDB database at the specified path using RocksDB backend.
    ///
    /// Creates a new database instance with RocksDB persistence at the given
    /// filesystem path. The namespace is set to "code_search" and database to "main".
    ///
    /// # Arguments
    /// * `path` - Filesystem path where RocksDB files will be stored
    ///
    /// # Errors
    /// Returns an error if the runtime cannot be created or if the database
    /// connection fails.
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let runtime = Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        let db = runtime.block_on(async {
            let db = Surreal::new::<RocksDb>(path)
                .await
                .map_err(|e| {
                    format!("Failed to connect to SurrealDB at {:?}: {}", path, e)
                })?;

            db.use_ns("code_search")
                .use_db("main")
                .await
                .map_err(|e| format!("Failed to select namespace/database: {}", e))?;

            Ok::<_, Box<dyn Error>>(db)
        })?;

        Ok(SurrealDatabase { db, runtime })
    }

    /// Opens an in-memory SurrealDB database for testing.
    ///
    /// Creates a new ephemeral database instance that stores data only in memory.
    /// The namespace is set to "code_search" and database to "main".
    ///
    /// # Errors
    /// Returns an error if the runtime cannot be created or if the database
    /// connection fails.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn open_mem() -> Result<Self, Box<dyn Error>> {
        let runtime = Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        let db = runtime.block_on(async {
            let db = Surreal::new::<Mem>(())
                .await
                .map_err(|e| format!("Failed to create in-memory SurrealDB: {}", e))?;

            db.use_ns("code_search")
                .use_db("main")
                .await
                .map_err(|e| format!("Failed to select namespace/database: {}", e))?;

            Ok::<_, Box<dyn Error>>(db)
        })?;

        Ok(SurrealDatabase { db, runtime })
    }
}

impl Database for SurrealDatabase {
    fn execute_query(
        &self,
        query: &str,
        params: QueryParams,
    ) -> Result<Box<dyn QueryResult>, Box<dyn Error>> {
        // Convert QueryParams to SurrealDB format
        let surreal_params = convert_params(params)?;

        // Execute query async via runtime
        let mut response = self.runtime.block_on(async {
            self.db
                .query(query)
                .bind(surreal_params)
                .await
                .map_err(|e| -> Box<dyn Error> { format!("SurrealDB query error: {}", e).into() })
        })?;

        // Take the first statement result
        // DDL statements (DEFINE TABLE, etc.) return None/empty results, so we handle that gracefully
        let result: Vec<surrealdb::sql::Value> = self.runtime.block_on(async {
            match response.take::<Vec<surrealdb::sql::Value>>(0) {
                Ok(values) => Ok::<Vec<surrealdb::sql::Value>, Box<dyn Error>>(values),
                Err(e) => {
                    // If deserialization fails (e.g., for DDL statements), return empty result
                    let err_str = e.to_string();
                    if err_str.contains("expected an enum variant") && err_str.contains("found None") {
                        Ok::<Vec<surrealdb::sql::Value>, Box<dyn Error>>(Vec::new())
                    } else {
                        Err(format!("Failed to extract results: {}", e).into())
                    }
                }
            }
        })?;

        // Extract headers from first object (if any)
        let headers = if let Some(surrealdb::sql::Value::Object(first)) = result.first() {
            first.keys().map(|k| k.to_string()).collect()
        } else {
            Vec::new()
        };

        // Convert each object to a row
        let rows: Vec<Box<dyn Row>> = result
            .into_iter()
            .map(|value| match value {
                surrealdb::sql::Value::Object(obj) => {
                    // Extract values in header order
                    let values: Vec<surrealdb::sql::Value> = headers
                        .iter()
                        .map(|h| {
                            obj.get(h)
                                .cloned()
                                .unwrap_or(surrealdb::sql::Value::None)
                        })
                        .collect();
                    Box::new(SurrealRow { values }) as Box<dyn Row>
                }
                _ => {
                    // Single value result
                    Box::new(SurrealRow {
                        values: vec![value],
                    }) as Box<dyn Row>
                }
            })
            .collect();

        Ok(Box::new(SurrealQueryResult { headers, rows }))
    }

    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self as &(dyn std::any::Any + Send + Sync)
    }
}

/// Converts QueryParams to SurrealDB's BTreeMap<String, surrealdb::sql::Value> format.
fn convert_params(
    params: QueryParams,
) -> Result<BTreeMap<String, surrealdb::sql::Value>, Box<dyn Error>> {
    let mut surreal_params = BTreeMap::new();

    for (key, value) in params.params().iter() {
        let surreal_value = match value {
            ValueType::Str(s) => surrealdb::sql::Value::Strand(s.clone().into()),
            ValueType::Int(i) => surrealdb::sql::Value::Number((*i).into()),
            ValueType::Float(f) => surrealdb::sql::Value::Number((*f).into()),
            ValueType::Bool(b) => surrealdb::sql::Value::Bool(*b),
        };
        surreal_params.insert(key.clone(), surreal_value);
    }

    Ok(surreal_params)
}

/// Query result wrapper implementing the generic QueryResult trait.
pub struct SurrealQueryResult {
    headers: Vec<String>,
    rows: Vec<Box<dyn Row>>,
}

impl QueryResult for SurrealQueryResult {
    fn headers(&self) -> &[String] {
        &self.headers
    }

    fn rows(&self) -> &[Box<dyn Row>] {
        &self.rows
    }

    fn into_rows(self: Box<Self>) -> Vec<Box<dyn Row>> {
        self.rows
    }
}

/// Row wrapper implementing the generic Row trait.
pub struct SurrealRow {
    values: Vec<surrealdb::sql::Value>,
}

impl Row for SurrealRow {
    fn get(&self, index: usize) -> Option<&dyn Value> {
        self.values.get(index).map(|v| v as &dyn Value)
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

/// Implements the Value trait for SurrealDB's sql::Value type.
impl Value for surrealdb::sql::Value {
    fn as_str(&self) -> Option<&str> {
        match self {
            surrealdb::sql::Value::Strand(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            surrealdb::sql::Value::Number(n) => Some(n.as_int()),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            surrealdb::sql::Value::Number(n) => Some(n.as_float()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            surrealdb::sql::Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_mem_compiles() {
        // Just verify it compiles, full testing in Ticket 06
        let _ = SurrealDatabase::open_mem();
    }

    #[test]
    fn test_parameter_conversion() {
        let params = QueryParams::new()
            .with_str("name", "test")
            .with_int("count", 42)
            .with_float("value", 3.14)
            .with_bool("flag", true);

        let surreal_params = convert_params(params).expect("Conversion should succeed");

        assert_eq!(surreal_params.len(), 4);
        assert!(surreal_params.contains_key("name"));
        assert!(surreal_params.contains_key("count"));
        assert!(surreal_params.contains_key("value"));
        assert!(surreal_params.contains_key("flag"));
    }

    #[test]
    fn test_value_extraction() {
        let str_val = surrealdb::sql::Value::Strand("test".into());
        assert_eq!(str_val.as_str(), Some("test"));

        let int_val = surrealdb::sql::Value::Number(42.into());
        assert_eq!(int_val.as_i64(), Some(42));

        let float_val = surrealdb::sql::Value::Number(3.14.into());
        assert_eq!(float_val.as_f64(), Some(3.14));

        let bool_val = surrealdb::sql::Value::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
    }
}
