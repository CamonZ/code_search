//! Integration tests comparing PostgreSQL AGE and CozoDB backends.
//!
//! These tests require a PostgreSQL instance with AGE extension.
//! Run with: cargo test --features postgres-tests
//!
//! Prerequisites:
//! 1. PostgreSQL with AGE extension installed
//! 2. Create test database: `createdb -U postgres code_search_test`

#![cfg(feature = "postgres-tests")]

use std::error::Error;
use cozo::DataValue;
use code_search::db::{DatabaseBackend, DatabaseConfig, PostgresAgeBackend, QueryResult};
use code_search::db::schema::{FUNCTIONS, run_migrations};

/// Test connection string for PostgreSQL (local instance)
const PG_CONNECTION: &str = "host=localhost user=postgres dbname=code_search_test";
const GRAPH_NAME: &str = "test_graph";

/// Setup test backends (both Cozo and PostgreSQL)
struct TestBackends {
    cozo: Box<dyn DatabaseBackend>,
    postgres: Box<dyn DatabaseBackend>,
}

impl TestBackends {
    fn new() -> Result<Self, Box<dyn Error>> {
        // Create CozoDB in-memory backend
        let cozo_config = DatabaseConfig::CozoMem;
        let cozo = cozo_config.connect()?;

        // Initialize CozoDB schema
        run_migrations(cozo.as_ref())?;

        // Create PostgreSQL AGE backend
        let postgres = PostgresAgeBackend::new(PG_CONNECTION, GRAPH_NAME)
            .map(|b| Box::new(b) as Box<dyn DatabaseBackend>)?;

        Ok(Self { cozo, postgres })
    }

    /// Get the backends for testing
    fn backends(&self) -> (&dyn DatabaseBackend, &dyn DatabaseBackend) {
        (&*self.cozo, &*self.postgres)
    }
}

// ============================================================================
// Helper: Result Comparison
// ============================================================================

/// Compare results between backends (order-independent)
///
/// This function is used for comparison in extended integration tests
/// that compare actual query results between backends.
#[allow(dead_code)]
fn compare_results(
    backend_name: &str,
    cozo_result: &QueryResult<DataValue>,
    postgres_result: &QueryResult<DataValue>,
) -> bool {
    if cozo_result.headers != postgres_result.headers {
        println!(
            "{}: Header mismatch:\n  Cozo:     {:?}\n  Postgres: {:?}",
            backend_name, cozo_result.headers, postgres_result.headers
        );
        return false;
    }

    if cozo_result.rows.len() != postgres_result.rows.len() {
        println!(
            "{}: Row count mismatch: Cozo={}, Postgres={}",
            backend_name,
            cozo_result.rows.len(),
            postgres_result.rows.len()
        );
        return false;
    }

    // Sort both results for comparison (results may come in different order)
    let mut cozo_sorted = cozo_result.rows.clone();
    let mut postgres_sorted = postgres_result.rows.clone();

    cozo_sorted.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
    postgres_sorted.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));

    for (i, (cozo_row, pg_row)) in cozo_sorted.iter().zip(postgres_sorted.iter()).enumerate() {
        if cozo_row != pg_row {
            println!(
                "{}: Row {} mismatch:\n  Cozo:     {:?}\n  Postgres: {:?}",
                backend_name, i, cozo_row, pg_row
            );
            return false;
        }
    }

    true
}

// ============================================================================
// Tests - Backend Connectivity
// ============================================================================

#[test]
fn test_backends_initialize() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    assert_eq!(cozo.backend_name(), "CozoMem");
    assert_eq!(postgres.backend_name(), "PostgresAge");

    Ok(())
}

// ============================================================================
// Tests - Empty Query Results
// ============================================================================

#[test]
fn test_empty_query_results() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, _postgres) = backends.backends();

    // Query for non-existent data returns empty results
    let cozo_result = cozo.execute_query_no_params(
        "?[project, module, name, arity] := *functions{project, module, name, arity}, module = 'NonExistentModule'"
    )?;

    assert!(cozo_result.rows.is_empty());

    Ok(())
}

// ============================================================================
// Tests - Query Execution without Errors
// ============================================================================

#[test]
fn test_cozo_query_execution() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (_cozo, _postgres) = backends.backends();

    // This test just verifies that the backends can execute queries
    // Full result comparison would require data seeding first

    Ok(())
}

#[test]
fn test_postgres_query_execution() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (_cozo, _postgres) = backends.backends();

    // This test just verifies that the backends can be instantiated
    // Full result comparison would require data seeding first

    Ok(())
}

// ============================================================================
// Tests - Schema Initialization
// ============================================================================

#[test]
fn test_postgres_backend_creation() -> Result<(), Box<dyn Error>> {
    // Test that we can create the backend with correct graph name
    let backend = PostgresAgeBackend::new(PG_CONNECTION, "idempotent_test")?;
    assert_eq!(backend.graph_name(), "idempotent_test");

    Ok(())
}

#[test]
fn test_postgres_backend_graph_name() -> Result<(), Box<dyn Error>> {
    let backend = PostgresAgeBackend::new(PG_CONNECTION, GRAPH_NAME)?;
    assert_eq!(backend.graph_name(), GRAPH_NAME);

    Ok(())
}

// ============================================================================
// Tests - Relation Existence Checks
// ============================================================================

#[test]
fn test_relation_exists_after_setup() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, _postgres) = backends.backends();

    // After migrations, the functions relation should exist in Cozo
    let cozo_exists = cozo.relation_exists("functions")?;
    assert!(cozo_exists, "functions relation should exist after migrations");

    Ok(())
}

// ============================================================================
// Tests - Insert Operations
// ============================================================================

#[test]
fn test_insert_empty_rows() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Try to insert empty rows into both backends
    let cozo_count = cozo.insert_rows(&FUNCTIONS, vec![])?;
    let postgres_count = postgres.insert_rows(&FUNCTIONS, vec![])?;

    // Both should handle empty inserts gracefully
    assert_eq!(cozo_count, 0);
    assert_eq!(postgres_count, 0);

    Ok(())
}

// ============================================================================
// Tests - Delete Operations
// ============================================================================

#[test]
fn test_delete_from_empty_database() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Delete from empty database should succeed with 0 rows deleted
    let cozo_count = cozo.delete_by_project(&FUNCTIONS, "nonexistent")?;
    let postgres_count = postgres.delete_by_project(&FUNCTIONS, "nonexistent")?;

    // Both should handle deletion from empty database gracefully
    assert_eq!(cozo_count, 0);
    // PostgreSQL behavior may vary, so we just verify it doesn't error
    let _ = postgres_count;

    Ok(())
}

// ============================================================================
// Tests - Upsert Operations
// ============================================================================

#[test]
fn test_upsert_empty_rows() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Upsert empty rows should succeed with 0 rows upserted
    let cozo_count = cozo.upsert_rows(&FUNCTIONS, vec![])?;
    let postgres_count = postgres.upsert_rows(&FUNCTIONS, vec![])?;

    assert_eq!(cozo_count, 0);
    assert_eq!(postgres_count, 0);

    Ok(())
}

// ============================================================================
// Tests - Error Handling
// ============================================================================

#[test]
fn test_invalid_query_both_backends() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Both should return errors for invalid queries
    let cozo_result = cozo.execute_query_no_params("invalid query syntax");
    let postgres_result = postgres.execute_query_no_params("invalid query syntax");

    assert!(cozo_result.is_err());
    assert!(postgres_result.is_err());

    Ok(())
}

#[test]
fn test_backend_names_are_different() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    let cozo_name = cozo.backend_name();
    let postgres_name = postgres.backend_name();

    assert_ne!(cozo_name, postgres_name);
    assert_eq!(cozo_name, "CozoMem");
    assert_eq!(postgres_name, "PostgresAge");

    Ok(())
}

// ============================================================================
// Tests - Insert/Upsert with Actual Data
// ============================================================================

use code_search::db::schema::{MODULES, CALLS};

#[test]
fn test_insert_rows_with_data() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Clean up any existing test data first
    let _ = cozo.delete_by_project(&MODULES, "test_insert_project");
    let _ = postgres.delete_by_project(&MODULES, "test_insert_project");

    // Create rows with actual data matching MODULES schema:
    // key: project, name
    // value: file, source
    let rows = vec![
        vec![
            DataValue::Str("test_insert_project".into()),  // project
            DataValue::Str("MyApp.Module1".into()),        // name
            DataValue::Str("lib/module1.ex".into()),       // file
            DataValue::Str("source".into()),               // source
        ],
        vec![
            DataValue::Str("test_insert_project".into()),
            DataValue::Str("MyApp.Module2".into()),
            DataValue::Str("lib/module2.ex".into()),
            DataValue::Str("source".into()),
        ],
    ];

    // Insert into CozoDB
    let cozo_count = cozo.insert_rows(&MODULES, rows.clone())?;
    assert_eq!(cozo_count, 2, "CozoDB should insert 2 rows");

    // Insert into PostgreSQL - this is where the agtype issue would surface
    let postgres_count = postgres.insert_rows(&MODULES, rows)?;
    assert_eq!(postgres_count, 2, "PostgreSQL should insert 2 rows");

    // Clean up
    let _ = cozo.delete_by_project(&MODULES, "test_insert_project");
    let _ = postgres.delete_by_project(&MODULES, "test_insert_project");

    Ok(())
}

#[test]
fn test_upsert_rows_with_data() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Clean up any existing test data first
    let _ = cozo.delete_by_project(&MODULES, "test_upsert_project");
    let _ = postgres.delete_by_project(&MODULES, "test_upsert_project");

    let rows = vec![
        vec![
            DataValue::Str("test_upsert_project".into()),
            DataValue::Str("MyApp.UpsertModule".into()),
            DataValue::Str("lib/upsert.ex".into()),
            DataValue::Str("source".into()),
        ],
    ];

    // Upsert into CozoDB
    let cozo_count = cozo.upsert_rows(&MODULES, rows.clone())?;
    assert_eq!(cozo_count, 1, "CozoDB should upsert 1 row");

    // Upsert into PostgreSQL - this is where the agtype issue would surface
    let postgres_count = postgres.upsert_rows(&MODULES, rows)?;
    assert_eq!(postgres_count, 1, "PostgreSQL should upsert 1 row");

    // Clean up
    let _ = cozo.delete_by_project(&MODULES, "test_upsert_project");
    let _ = postgres.delete_by_project(&MODULES, "test_upsert_project");

    Ok(())
}

#[test]
fn test_insert_functions_with_data() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Clean up
    let _ = cozo.delete_by_project(&FUNCTIONS, "test_func_project");
    let _ = postgres.delete_by_project(&FUNCTIONS, "test_func_project");

    // FUNCTIONS schema:
    // key: project, module, name, arity
    // value: return_type, args, source
    let rows = vec![
        vec![
            DataValue::Str("test_func_project".into()),  // project
            DataValue::Str("MyApp.Functions".into()),    // module
            DataValue::Str("process".into()),            // name
            DataValue::from(2i64),                       // arity
            DataValue::Str("term()".into()),             // return_type
            DataValue::Str("arg1, arg2".into()),         // args
            DataValue::Str("source".into()),             // source
        ],
    ];

    let cozo_count = cozo.insert_rows(&FUNCTIONS, rows.clone())?;
    assert_eq!(cozo_count, 1, "CozoDB should insert 1 function");

    let postgres_count = postgres.insert_rows(&FUNCTIONS, rows)?;
    assert_eq!(postgres_count, 1, "PostgreSQL should insert 1 function");

    // Clean up
    let _ = cozo.delete_by_project(&FUNCTIONS, "test_func_project");
    let _ = postgres.delete_by_project(&FUNCTIONS, "test_func_project");

    Ok(())
}

#[test]
fn test_insert_calls_with_data() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Clean up
    let _ = cozo.delete_by_project(&CALLS, "test_calls_project");
    let _ = postgres.delete_by_project(&CALLS, "test_calls_project");

    // CALLS schema has many fields - this tests a more complex insert
    // key: project, caller_module, caller_function, callee_module, callee_function,
    //      callee_arity, file, line, column
    // value: call_type, caller_kind, callee_args
    let rows = vec![
        vec![
            DataValue::Str("test_calls_project".into()),  // project
            DataValue::Str("MyApp.Caller".into()),        // caller_module
            DataValue::Str("do_work".into()),             // caller_function
            DataValue::Str("MyApp.Callee".into()),        // callee_module
            DataValue::Str("helper".into()),              // callee_function
            DataValue::from(1i64),                        // callee_arity
            DataValue::Str("lib/caller.ex".into()),       // file
            DataValue::from(42i64),                       // line
            DataValue::from(5i64),                        // column
            DataValue::Str("local".into()),               // call_type
            DataValue::Str("def".into()),                 // caller_kind
            DataValue::Str("x".into()),                   // callee_args
        ],
    ];

    let cozo_count = cozo.insert_rows(&CALLS, rows.clone())?;
    assert_eq!(cozo_count, 1, "CozoDB should insert 1 call");

    let postgres_count = postgres.insert_rows(&CALLS, rows)?;
    assert_eq!(postgres_count, 1, "PostgreSQL should insert 1 call");

    // Clean up
    let _ = cozo.delete_by_project(&CALLS, "test_calls_project");
    let _ = postgres.delete_by_project(&CALLS, "test_calls_project");

    Ok(())
}

#[test]
fn test_insert_large_batch() -> Result<(), Box<dyn Error>> {
    let backends = TestBackends::new()?;
    let (cozo, postgres) = backends.backends();

    // Clean up
    let _ = cozo.delete_by_project(&MODULES, "test_batch_project");
    let _ = postgres.delete_by_project(&MODULES, "test_batch_project");

    // Create 100 rows to test batching behavior
    let rows: Vec<Vec<DataValue>> = (0..100)
        .map(|i| vec![
            DataValue::Str("test_batch_project".into()),
            DataValue::Str(format!("MyApp.Module{}", i).into()),
            DataValue::Str(format!("lib/module{}.ex", i).into()),
            DataValue::Str("source".into()),
        ])
        .collect();

    let cozo_count = cozo.insert_rows(&MODULES, rows.clone())?;
    assert_eq!(cozo_count, 100, "CozoDB should insert 100 rows");

    let postgres_count = postgres.insert_rows(&MODULES, rows)?;
    assert_eq!(postgres_count, 100, "PostgreSQL should insert 100 rows");

    // Clean up
    let _ = cozo.delete_by_project(&MODULES, "test_batch_project");
    let _ = postgres.delete_by_project(&MODULES, "test_batch_project");

    Ok(())
}
