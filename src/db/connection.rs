//! Database connection management.

use std::error::Error;
use std::path::Path;

use cozo::{DataValue, DbInstance, NamedRows, ScriptMutability};

use super::backend::{DatabaseBackend, Params, QueryResult};
use super::escape::escape_string;
use super::schema::{SchemaRelation, CozoCompiler};
#[cfg(test)]
use super::schema::run_migrations;
use super::DbError;

/// Format a row of DataValues as a Cozo array literal.
///
/// Converts each DataValue to its Cozo representation:
/// - String: `"value"` (double quotes, escaped)
/// - Int: `123`
/// - Float: `1.23`
/// - Bool: `true` or `false`
/// - Null: `null`
/// - List: `[item1, item2, ...]` (nested, with items formatted recursively)
///
/// # Example
/// ```ignore
/// let row = vec![
///     DataValue::Str("MyApp".into()),
///     DataValue::Num(cozo::Num::Int(42)),
///     DataValue::Bool(true),
/// ];
/// assert_eq!(format_cozo_row(&row), r#"["MyApp", 42, true]"#);
/// ```
fn format_cozo_row(row: &[DataValue]) -> String {
    let values: Vec<String> = row.iter().map(format_cozo_value).collect();
    format!("[{}]", values.join(", "))
}

/// Format a single DataValue for use in Cozo literals.
fn format_cozo_value(v: &DataValue) -> String {
    match v {
        DataValue::Str(s) => format!(r#""{}""#, escape_string(s)),
        DataValue::Num(n) => match n {
            cozo::Num::Int(i) => i.to_string(),
            cozo::Num::Float(f) => f.to_string(),
        },
        DataValue::Bool(b) => b.to_string(),
        DataValue::Null => "null".to_string(),
        DataValue::List(l) => {
            let items: Vec<String> = l.iter().map(format_cozo_value).collect();
            format!("[{}]", items.join(", "))
        }
        _ => "null".to_string(), // Fallback for other types
    }
}

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
    fn execute_query(&self, script: &str, params: &Params) -> Result<QueryResult, Box<dyn Error>> {
        let result = self
            .db
            .run_script(script, params.clone(), ScriptMutability::Mutable)?;
        Ok(named_rows_to_query_result(result))
    }

    fn backend_name(&self) -> &'static str {
        "CozoSqlite"
    }

    fn relation_exists(&self, name: &str) -> Result<bool, Box<dyn Error>> {
        // Use the ::relations system command to list all relations
        // and check if the target relation exists
        let result = self.execute_query_no_params("::relations")?;

        // The ::relations command returns rows with relation names
        // Check if any row contains our relation name
        for row in &result.rows {
            if let Some(DataValue::Str(relation_name)) = row.first() {
                let relation_str: &str = relation_name.as_ref();
                if relation_str == name {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn try_create_relation(&self, schema: &str) -> Result<bool, Box<dyn Error>> {
        match self.execute_query_no_params(schema) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("AlreadyExists")
                    || err_str.contains("stored_relation_conflict")
                    || err_str.contains("conflicts")
                {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn insert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        if rows.is_empty() {
            return Ok(0);
        }

        let row_count = rows.len();

        // Convert rows to Cozo row literal format
        let row_literals: Vec<String> = rows.iter().map(|row| format_cozo_row(row)).collect();

        // Chunk for large imports
        const CHUNK_SIZE: usize = 500;
        for chunk in row_literals.chunks(CHUNK_SIZE) {
            let script = CozoCompiler::compile_insert(relation, &chunk.to_vec());
            self.execute_query_no_params(&script)?;
        }

        Ok(row_count)
    }

    fn delete_by_project(
        &self,
        relation: &SchemaRelation,
        project: &str,
    ) -> Result<usize, Box<dyn Error>> {
        let script = CozoCompiler::compile_delete_by_project(relation);
        let mut params = Params::new();
        params.insert("project".to_string(), DataValue::Str(project.into()));
        self.execute_query(&script, &params)?;
        Ok(0) // Cozo doesn't return delete count
    }

    fn upsert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        // Cozo :put is already an upsert
        self.insert_rows(relation, rows)
    }

    fn as_db_instance(&self) -> &cozo::DbInstance {
        &self.db
    }
}

/// CozoDB backend using in-memory storage.
///
/// Primarily used for testing and configuration. Production code
/// uses CozoSqliteBackend via `open_db()`.
pub struct CozoMemBackend {
    db: DbInstance,
}

impl CozoMemBackend {
    /// Create a new in-memory CozoDB backend from a database instance.
    pub fn new(db: DbInstance) -> Self {
        Self { db }
    }
}

impl DatabaseBackend for CozoMemBackend {
    fn execute_query(&self, script: &str, params: &Params) -> Result<QueryResult, Box<dyn Error>> {
        let result = self
            .db
            .run_script(script, params.clone(), ScriptMutability::Mutable)?;
        Ok(named_rows_to_query_result(result))
    }

    fn backend_name(&self) -> &'static str {
        "CozoMem"
    }

    fn relation_exists(&self, name: &str) -> Result<bool, Box<dyn Error>> {
        // Use the ::relations system command to list all relations
        // and check if the target relation exists
        let result = self.execute_query_no_params("::relations")?;

        // The ::relations command returns rows with relation names
        // Check if any row contains our relation name
        for row in &result.rows {
            if let Some(DataValue::Str(relation_name)) = row.first() {
                let relation_str: &str = relation_name.as_ref();
                if relation_str == name {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn try_create_relation(&self, schema: &str) -> Result<bool, Box<dyn Error>> {
        match self.execute_query_no_params(schema) {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("AlreadyExists")
                    || err_str.contains("stored_relation_conflict")
                    || err_str.contains("conflicts")
                {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn insert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        if rows.is_empty() {
            return Ok(0);
        }

        let row_count = rows.len();

        // Convert rows to Cozo row literal format
        let row_literals: Vec<String> = rows.iter().map(|row| format_cozo_row(row)).collect();

        // Chunk for large imports
        const CHUNK_SIZE: usize = 500;
        for chunk in row_literals.chunks(CHUNK_SIZE) {
            let script = CozoCompiler::compile_insert(relation, &chunk.to_vec());
            self.execute_query_no_params(&script)?;
        }

        Ok(row_count)
    }

    fn delete_by_project(
        &self,
        relation: &SchemaRelation,
        project: &str,
    ) -> Result<usize, Box<dyn Error>> {
        let script = CozoCompiler::compile_delete_by_project(relation);
        let mut params = Params::new();
        params.insert("project".to_string(), DataValue::Str(project.into()));
        self.execute_query(&script, &params)?;
        Ok(0) // Cozo doesn't return delete count
    }

    fn upsert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        // Cozo :put is already an upsert
        self.insert_rows(relation, rows)
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
#[allow(dead_code)] // Will be used after Ticket #44
pub fn open_db(path: &Path) -> Result<Box<dyn DatabaseBackend>, Box<dyn Error>> {
    let db = DbInstance::new("sqlite", path, "").map_err(|e| {
        Box::new(DbError::OpenFailed {
            path: path.display().to_string(),
            message: format!("{:?}", e),
        }) as Box<dyn Error>
    })?;
    let backend = Box::new(CozoSqliteBackend::new(db));

    Ok(backend)
}

/// Create an in-memory database backend for test utilities.
///
/// Returns a boxed DatabaseBackend trait object wrapping an in-memory CozoDB instance.
/// Used by test fixtures after Ticket #44.
#[cfg(test)]
pub fn open_mem_db(do_run_migrations: bool) -> Result<Box<dyn DatabaseBackend>, Box<dyn Error>> {
    let db = DbInstance::new("mem", "", "").map_err(|e| {
        Box::new(DbError::OpenFailed {
            path: "mem".to_string(),
            message: format!("{:?}", e),
        }) as Box<dyn Error>
    })?;
    let backend = Box::new(CozoMemBackend::new(db));

    if do_run_migrations {
        // Run migrations to ensure schema exists
        run_migrations(backend.as_ref())?;
    }

    Ok(backend)
}

/// Create an in-memory database backend WITHOUT running migrations.
///
/// Used only for testing empty database scenarios.
/// WARNING: Do not use in production code.
#[cfg(test)]
pub fn open_mem_db_empty() -> Result<Box<dyn DatabaseBackend>, Box<dyn Error>> {
    let db = DbInstance::new("mem", "", "").map_err(|e| {
        Box::new(DbError::OpenFailed {
            path: "mem".to_string(),
            message: format!("{:?}", e),
        }) as Box<dyn Error>
    })?;
    Ok(Box::new(CozoMemBackend::new(db)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::MODULES;

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

    #[test]
    fn test_format_cozo_row_strings() {
        let row = vec![
            DataValue::Str("MyApp".into()),
            DataValue::Str("test".into()),
        ];
        let formatted = format_cozo_row(&row);
        assert_eq!(formatted, r#"["MyApp", "test"]"#);
    }

    #[test]
    fn test_format_cozo_row_mixed() {
        let row = vec![
            DataValue::Str("proj".into()),
            DataValue::Num(cozo::Num::Int(42)),
            DataValue::Bool(true),
        ];
        let formatted = format_cozo_row(&row);
        assert_eq!(formatted, r#"["proj", 42, true]"#);
    }

    #[test]
    fn test_format_cozo_row_escaping() {
        let row = vec![
            DataValue::Str(r#"value with "quotes""#.into()),
        ];
        let formatted = format_cozo_row(&row);
        assert!(formatted.contains(r#"\""#) || formatted.contains(r#""""#));
    }

    #[test]
    fn test_format_cozo_row_null() {
        let row = vec![
            DataValue::Str("test".into()),
            DataValue::Null,
        ];
        let formatted = format_cozo_row(&row);
        assert_eq!(formatted, r#"["test", null]"#);
    }

    #[test]
    fn test_insert_rows_empty() {
        let db = open_mem_db(true).unwrap();
        let result = db.insert_rows(&MODULES, vec![]).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_insert_rows_modules() {
        let db = open_mem_db(true).unwrap();

        let rows = vec![
            vec![
                DataValue::Str("test_proj".into()),
                DataValue::Str("MyApp".into()),
                DataValue::Str("".into()),
                DataValue::Str("unknown".into()),
            ],
        ];

        let result = db.insert_rows(&MODULES, rows).unwrap();
        assert_eq!(result, 1);

        // Verify data was inserted
        let query_result = db.execute_query_no_params(
            "?[project, name] := *modules{project, name}"
        ).unwrap();
        assert_eq!(query_result.rows.len(), 1);
    }

    #[test]
    fn test_delete_by_project() {
        let db = open_mem_db(true).unwrap();

        // Insert some data
        let rows = vec![
            vec![
                DataValue::Str("proj1".into()),
                DataValue::Str("MyApp".into()),
                DataValue::Str("".into()),
                DataValue::Str("unknown".into()),
            ],
            vec![
                DataValue::Str("proj2".into()),
                DataValue::Str("OtherApp".into()),
                DataValue::Str("".into()),
                DataValue::Str("unknown".into()),
            ],
        ];
        db.insert_rows(&MODULES, rows).unwrap();

        // Verify initial count
        let before = db.execute_query_no_params(
            "?[project, name] := *modules{project, name}"
        ).unwrap();
        assert_eq!(before.rows.len(), 2);

        // Delete proj1 - this should succeed without error
        let result = db.delete_by_project(&MODULES, "proj1");
        assert!(result.is_ok(), "delete_by_project should succeed");

        // Verify deletion happened (rows decreased)
        let after = db.execute_query_no_params(
            "?[project, name] := *modules{project, name}"
        ).unwrap();
        // The row count should be less than before deletion
        assert!(after.rows.len() < before.rows.len(),
            "Row count should decrease after deletion (before: {}, after: {})",
            before.rows.len(),
            after.rows.len());
    }

    #[test]
    fn test_upsert_rows_insert() {
        let db = open_mem_db(true).unwrap();

        let rows = vec![
            vec![
                DataValue::Str("test_proj".into()),
                DataValue::Str("MyApp".into()),
                DataValue::Str("lib/app.ex".into()),
                DataValue::Str("unknown".into()),
            ],
        ];

        let result = db.upsert_rows(&MODULES, rows).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_upsert_rows_update() {
        let db = open_mem_db(true).unwrap();

        // Insert initial data
        let initial = vec![
            vec![
                DataValue::Str("test_proj".into()),
                DataValue::Str("MyApp".into()),
                DataValue::Str("".into()),
                DataValue::Str("unknown".into()),
            ],
        ];
        db.insert_rows(&MODULES, initial).unwrap();

        // Upsert with updated file
        let updated = vec![
            vec![
                DataValue::Str("test_proj".into()),
                DataValue::Str("MyApp".into()),
                DataValue::Str("lib/app.ex".into()), // Updated
                DataValue::Str("elixir".into()), // Updated
            ],
        ];
        db.upsert_rows(&MODULES, updated).unwrap();

        // Verify update (still 1 row, but with new values)
        let query_result = db.execute_query_no_params(
            "?[project, name, file, source] := *modules{project, name, file, source}"
        ).unwrap();
        assert_eq!(query_result.rows.len(), 1);
    }

    #[test]
    fn test_insert_rows_chunking() {
        let db = open_mem_db(true).unwrap();

        // Create more rows than CHUNK_SIZE to test chunking
        let rows: Vec<Vec<DataValue>> = (0..600)
            .map(|i| vec![
                DataValue::Str("test_proj".into()),
                DataValue::Str(format!("Module{}", i).into()),
                DataValue::Str("".into()),
                DataValue::Str("unknown".into()),
            ])
            .collect();

        let result = db.insert_rows(&MODULES, rows).unwrap();
        assert_eq!(result, 600);

        // Verify all were inserted by checking that we can retrieve them
        let query_result = db.execute_query_no_params(
            "?[project, name] := *modules{project, name}"
        ).unwrap();
        // Should have successfully inserted all 600 rows
        assert_eq!(query_result.rows.len(), 600, "All 600 rows should be inserted and retrievable");
    }
}
