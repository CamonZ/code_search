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

        // Execute query and extract results in a single async block
        // This ensures the transaction completes properly before we return
        let result: Vec<surrealdb::sql::Value> = self.runtime.block_on(async {
            let response = self.db
                .query(query)
                .bind(surreal_params)
                .await
                .map_err(|e| -> Box<dyn Error> { format!("SurrealDB query error: {}", e).into() })?;

            // Check for errors - this is critical for transaction completion
            // Note: check() consumes and returns the Response
            let mut response = response.check().map_err(|e| -> Box<dyn Error> {
                format!("SurrealDB query validation error: {}", e).into()
            })?;

            // Take the first statement result as surrealdb::Value
            // The Response from SurrealDB contains results for each statement in the query
            // Each result can be: None (DDL), single object, or array of objects
            let raw_result: Result<surrealdb::Value, _> = response.take(0);

            match raw_result {
                Ok(value) => {
                    // Convert surrealdb::Value to surrealdb::sql::Value via JSON
                    // This is necessary because surrealdb::Value wraps surrealdb::sql::Value
                    // but the wrapper's inner field is private
                    let json_str = serde_json::to_string(&value)
                        .map_err(|e| format!("Failed to serialize Value to JSON: {}", e))?;

                    let sql_value: surrealdb::sql::Value = serde_json::from_str(&json_str)
                        .map_err(|e| format!("Failed to deserialize JSON to sql::Value: {}", e))?;

                    // Handle the three cases: Array, Object, or None
                    match sql_value {
                        surrealdb::sql::Value::Array(arr) => {
                            // SELECT queries return arrays
                            Ok::<Vec<surrealdb::sql::Value>, Box<dyn Error>>(arr.0)
                        },
                        surrealdb::sql::Value::Object(_) => {
                            // INFO commands and some other queries return single objects
                            Ok::<Vec<surrealdb::sql::Value>, Box<dyn Error>>(vec![sql_value])
                        },
                        surrealdb::sql::Value::None => {
                            // DDL statements (DEFINE, CREATE without results) return None
                            Ok::<Vec<surrealdb::sql::Value>, Box<dyn Error>>(Vec::new())
                        },
                        other => {
                            // Unexpected types - wrap in Vec to be safe
                            Ok::<Vec<surrealdb::sql::Value>, Box<dyn Error>>(vec![other])
                        }
                    }
                },
                Err(e) => {
                    Err(format!("Failed to extract results: {}", e).into())
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

    // ==================== In-Memory Database Tests ====================

    #[test]
    fn test_open_mem() {
        let db = SurrealDatabase::open_mem().expect("Failed to open in-memory database");
        // Verify database is usable by executing a simple DDL statement
        let result = db.execute_query("DEFINE TABLE test SCHEMAFULL;", QueryParams::new());
        assert!(result.is_ok());
    }

    // ==================== Parameter Conversion Tests ====================

    #[test]
    fn test_parameter_conversion_str() {
        let params = QueryParams::new().with_str("name", "test");
        let converted = convert_params(params).expect("Failed to convert params");
        assert!(converted.contains_key("name"));

        // Verify the value is correctly converted to a Strand
        if let Some(surrealdb::sql::Value::Strand(s)) = converted.get("name") {
            assert_eq!(s.as_str(), "test");
        } else {
            panic!("Expected Strand value");
        }
    }

    #[test]
    fn test_parameter_conversion_int() {
        let params = QueryParams::new().with_int("count", 42);
        let converted = convert_params(params).expect("Failed to convert params");
        assert!(converted.contains_key("count"));

        // Verify the value is correctly converted to a Number
        if let Some(surrealdb::sql::Value::Number(n)) = converted.get("count") {
            assert_eq!(n.as_int(), 42);
        } else {
            panic!("Expected Number value");
        }
    }

    #[test]
    fn test_parameter_conversion_float() {
        let params = QueryParams::new().with_float("price", 3.14);
        let converted = convert_params(params).expect("Failed to convert params");
        assert!(converted.contains_key("price"));

        // Verify the value is correctly converted to a Number
        if let Some(surrealdb::sql::Value::Number(n)) = converted.get("price") {
            let f = n.as_float();
            assert!((f - 3.14).abs() < 0.01);
        } else {
            panic!("Expected Number value");
        }
    }

    #[test]
    fn test_parameter_conversion_bool() {
        let params = QueryParams::new().with_bool("active", true);
        let converted = convert_params(params).expect("Failed to convert params");
        assert!(converted.contains_key("active"));

        // Verify the value is correctly converted to a Bool
        if let Some(surrealdb::sql::Value::Bool(b)) = converted.get("active") {
            assert_eq!(*b, true);
        } else {
            panic!("Expected Bool value");
        }
    }

    #[test]
    fn test_parameter_conversion_multiple_types() {
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

    // ==================== Value Extraction Tests ====================

    #[test]
    fn test_value_extraction_str() {
        let val = surrealdb::sql::Value::Strand("hello".into());
        assert_eq!(val.as_str(), Some("hello"));
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_bool(), None);
        assert_eq!(val.as_f64(), None);
    }

    #[test]
    fn test_value_extraction_int() {
        let val = surrealdb::sql::Value::Number(42.into());
        assert_eq!(val.as_i64(), Some(42));
        assert_eq!(val.as_str(), None);
        assert_eq!(val.as_bool(), None);
    }

    #[test]
    fn test_value_extraction_float() {
        let val = surrealdb::sql::Value::Number(3.14.into());
        assert!(val.as_f64().is_some());
        let f = val.as_f64().unwrap();
        assert!((f - 3.14).abs() < 0.01);
        assert_eq!(val.as_str(), None);
        assert_eq!(val.as_bool(), None);
    }

    #[test]
    fn test_value_extraction_bool() {
        let val = surrealdb::sql::Value::Bool(true);
        assert_eq!(val.as_bool(), Some(true));
        assert_eq!(val.as_i64(), None);
        assert_eq!(val.as_str(), None);
    }

    #[test]
    fn test_value_extraction_bool_false() {
        let val = surrealdb::sql::Value::Bool(false);
        assert_eq!(val.as_bool(), Some(false));
    }

    // ==================== Query Execution Tests ====================

    #[test]
    fn test_schema_creation() {
        let db = SurrealDatabase::open_mem().expect("Failed to open database");

        // Test creating a simple table with SCHEMAFULL
        let result = db.execute_query(
            "DEFINE TABLE test_table SCHEMAFULL; DEFINE FIELD name ON test_table TYPE string;",
            QueryParams::new(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_statements() {
        let db = SurrealDatabase::open_mem().expect("Failed to open database");

        // Test executing multiple DDL statements in one query
        let result = db.execute_query(
            "DEFINE TABLE users SCHEMAFULL; DEFINE FIELD username ON users TYPE string;",
            QueryParams::new(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parameterized_query() {
        let db = SurrealDatabase::open_mem().expect("Failed to open database");

        // Create table
        db.execute_query(
            "DEFINE TABLE config SCHEMAFULL; DEFINE FIELD key ON config TYPE string; DEFINE FIELD value ON config TYPE string;",
            QueryParams::new(),
        )
        .expect("Failed to create table");

        // Test parameter conversion in query
        let params = QueryParams::new()
            .with_str("key", "setting1")
            .with_str("value", "enabled");

        // Just test that parameters are accepted without error
        let result = db.execute_query(
            "DEFINE TABLE test_with_params SCHEMAFULL; DEFINE FIELD key ON test_with_params TYPE string;",
            params,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_database_trait_implementation() {
        let db = SurrealDatabase::open_mem().expect("Failed to open database");

        // Verify the Database trait is properly implemented
        let result = db.execute_query(
            "DEFINE TABLE trait_test SCHEMAFULL;",
            QueryParams::new(),
        );
        assert!(result.is_ok());

        // Verify as_any() works
        let any_ref = db.as_any();
        assert!(any_ref.is::<SurrealDatabase>());
    }

    // ==================== Persistent Database Tests ====================

    #[test]
    fn test_open_persistent_database() {
        use tempfile::tempdir;

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test_persistent.db");

        // Test opening a persistent database
        let db = SurrealDatabase::open(&db_path).expect("Failed to open persistent database");

        // Verify database is usable
        let result = db.execute_query(
            "DEFINE TABLE persistent_test SCHEMAFULL;",
            QueryParams::new(),
        );
        assert!(result.is_ok(), "Database should be usable after opening");
    }

    // ==================== QueryResult Trait Tests ====================

    #[test]
    fn test_query_result_trait() {
        let db = SurrealDatabase::open_mem().expect("Failed to open database");

        // Create multiple tables to get a result with multiple rows
        let result = db
            .execute_query(
                "DEFINE TABLE test1 SCHEMAFULL; DEFINE TABLE test2 SCHEMAFULL;",
                QueryParams::new(),
            )
            .expect("Failed to create tables");

        // Test headers() - DDL returns empty headers
        let headers = result.headers();
        assert!(headers.is_empty(), "DDL statements return no headers");

        // Test rows() - DDL returns empty rows
        let rows = result.rows();
        assert_eq!(rows.len(), 0, "DDL statements return no rows");

        // Test into_rows()
        let rows_vec = result.into_rows();
        assert_eq!(rows_vec.len(), 0, "Should have same count after into_rows");
    }

    // ==================== Row Trait Tests ====================

    #[test]
    fn test_row_trait() {
        // Test Row trait methods by creating a SurrealRow directly
        use surrealdb::sql::Value as SurrealValue;

        let values = vec![
            SurrealValue::Strand("test".into()),
            SurrealValue::Number(42.into()),
            SurrealValue::Bool(true),
        ];

        let row = SurrealRow { values };

        // Test len()
        assert_eq!(row.len(), 3, "Row should have 3 columns");

        // Test get()
        let first_value = row.get(0);
        assert!(first_value.is_some(), "Should be able to get first column");
        assert_eq!(first_value.unwrap().as_str(), Some("test"));

        let second_value = row.get(1);
        assert!(second_value.is_some(), "Should be able to get second column");
        assert_eq!(second_value.unwrap().as_i64(), Some(42));

        let third_value = row.get(2);
        assert!(third_value.is_some(), "Should be able to get third column");
        assert_eq!(third_value.unwrap().as_bool(), Some(true));

        // Test is_empty()
        assert!(!row.is_empty(), "Row should not be empty");

        // Test get() with out of bounds index
        let out_of_bounds = row.get(999);
        assert!(out_of_bounds.is_none(), "Out of bounds get should return None");

        // Test empty row
        let empty_row = SurrealRow { values: vec![] };
        assert!(empty_row.is_empty(), "Empty row should be empty");
        assert_eq!(empty_row.len(), 0, "Empty row length should be 0");
    }
}
