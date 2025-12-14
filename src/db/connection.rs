//! Database connection management.

use std::error::Error;
use std::path::Path;

use cozo::{DbInstance, NamedRows, ScriptMutability};

use super::backend::{DatabaseBackend, Params, QueryResult};
use super::DbError;

/// CozoDB backend using SQLite storage.
pub struct CozoSqliteBackend {
    db: DbInstance,
}

impl CozoSqliteBackend {
    /// Create a new SQLite-backed CozoDB backend from a database instance.
    pub fn new(db: DbInstance) -> Self {
        Self { db }
    }
}

impl DatabaseBackend for CozoSqliteBackend {
    fn execute_query(
        &self,
        script: &str,
        params: &Params,
    ) -> Result<QueryResult, Box<dyn Error>> {
        let result = self.db.run_script(script, params.clone(), ScriptMutability::Mutable)?;
        Ok(named_rows_to_query_result(result))
    }

    fn backend_name(&self) -> &'static str {
        "CozoSqlite"
    }

    fn relation_exists(&self, name: &str) -> Result<bool, Box<dyn Error>> {
        let script = format!("?[count] := *{}", name);
        match self.execute_query_no_params(&script) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("NoSuchRelation") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn try_create_relation(&self, schema: &str) -> Result<bool, Box<dyn Error>> {
        match self.execute_query_no_params(schema) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("AlreadyExists") || err_str.contains("stored_relation_conflict") || err_str.contains("conflicts") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn as_db_instance(&self) -> &cozo::DbInstance {
        &self.db
    }
}

/// CozoDB backend using in-memory storage (test-only).
///
/// Used to test the DatabaseBackend trait implementation. Production code
/// uses CozoSqliteBackend via `open_db()`.
#[cfg(test)]
pub struct CozoMemBackend {
    db: DbInstance,
}

#[cfg(test)]
impl CozoMemBackend {
    /// Create a new in-memory CozoDB backend from a database instance.
    pub fn new(db: DbInstance) -> Self {
        Self { db }
    }
}

#[cfg(test)]
impl DatabaseBackend for CozoMemBackend {
    fn execute_query(
        &self,
        script: &str,
        params: &Params,
    ) -> Result<QueryResult, Box<dyn Error>> {
        let result = self.db.run_script(script, params.clone(), ScriptMutability::Mutable)?;
        Ok(named_rows_to_query_result(result))
    }

    fn backend_name(&self) -> &'static str {
        "CozoMem"
    }

    fn relation_exists(&self, name: &str) -> Result<bool, Box<dyn Error>> {
        let script = format!("?[count] := *{}", name);
        match self.execute_query_no_params(&script) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("NoSuchRelation") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn try_create_relation(&self, schema: &str) -> Result<bool, Box<dyn Error>> {
        match self.execute_query_no_params(schema) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("AlreadyExists") || err_str.contains("stored_relation_conflict") || err_str.contains("conflicts") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn as_db_instance(&self) -> &cozo::DbInstance {
        &self.db
    }
}

/// Helper to convert CozoDB NamedRows to our QueryResult type.
fn named_rows_to_query_result(rows: NamedRows) -> QueryResult {
    QueryResult {
        headers: rows.headers.iter().map(|h| h.to_string()).collect(),
        rows: rows.rows,
    }
}

/// Open a CozoDB database backed by SQLite storage.
pub fn open_db(path: &Path) -> Result<Box<dyn DatabaseBackend>, Box<dyn Error>> {
    let db = DbInstance::new("sqlite", path, "").map_err(|e| {
        Box::new(DbError::OpenFailed {
            path: path.display().to_string(),
            message: format!("{:?}", e),
        }) as Box<dyn Error>
    })?;
    Ok(Box::new(CozoSqliteBackend::new(db)))
}

/// Create a raw in-memory DbInstance for test utilities.
///
/// Used until Ticket #44 updates Execute trait to accept `&dyn DatabaseBackend`.
#[cfg(test)]
pub fn open_mem_db_raw() -> DbInstance {
    DbInstance::new("mem", "", "").expect("Failed to create in-memory DB")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cozosqlite_backend_name() {
        let db = DbInstance::new("mem", "", "").expect("Failed to create test DB");
        let backend = CozoSqliteBackend::new(db);
        assert_eq!(backend.backend_name(), "CozoSqlite");
    }

    #[test]
    fn test_cozomem_backend_name() {
        let db = DbInstance::new("mem", "", "").expect("Failed to create test DB");
        let backend = CozoMemBackend::new(db);
        assert_eq!(backend.backend_name(), "CozoMem");
    }

    #[test]
    fn test_execute_query_no_params_works() {
        let db = DbInstance::new("mem", "", "").expect("Failed to create test DB");
        let backend = CozoMemBackend::new(db);

        // Execute a simple query that returns data without needing to create relations
        let query_script = r#"?[x] := x in [1, 2, 3]"#;
        let result = backend
            .execute_query_no_params(query_script)
            .expect("Failed to execute query");

        assert_eq!(result.headers.len(), 1);
        assert_eq!(result.rows.len(), 3);
    }

    #[test]
    fn test_try_create_relation_idempotent() {
        let db = DbInstance::new("mem", "", "").expect("Failed to create test DB");
        let backend = CozoMemBackend::new(db);

        let schema = r#":create test_table {x: Int}"#;

        // First creation should return true
        let first_result = backend
            .try_create_relation(schema)
            .expect("Failed on first creation");
        assert!(first_result);

        // Second creation should return false (already exists)
        let second_result = backend
            .try_create_relation(schema)
            .expect("Failed on second creation");
        assert!(!second_result);
    }
}
