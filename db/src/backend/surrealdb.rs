//! SurrealDB backend implementation.
//!
//! This module provides the SurrealDB-specific implementation of the Database trait,
//! wrapping the async SurrealDB API with a synchronous interface using tokio::Runtime.

use super::{Database, QueryParams, QueryResult, ValueType};
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
        let _result = self.runtime.block_on(async {
            self.db
                .query(query)
                .bind(surreal_params)
                .await
                .map_err(|e| -> Box<dyn Error> { format!("SurrealDB query error: {}", e).into() })
        })?;

        // Result wrapping will be implemented in Ticket 03
        unimplemented!("Result wrapping - implemented in Ticket 03")
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
}
