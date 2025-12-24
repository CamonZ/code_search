#![cfg(feature = "backend-surrealdb")]

//! Integration tests for SurrealDB backend.
//!
//! These tests verify end-to-end functionality of the SurrealDB backend,
//! including database connection, schema creation, and query execution.

use db::backend::{open_database, QueryParams};
use db::open_mem_db;
use db::queries::schema::{create_schema, relation_names};
use tempfile::tempdir;

// ==================== Schema Creation Tests ====================

#[test]
fn test_setup_command_with_backend() {
    let db = open_mem_db().expect("Failed to open database");
    let result = create_schema(db.as_ref()).expect("Failed to create schema");

    // Should create 9 tables (5 nodes + 4 relationships)
    assert_eq!(
        result.len(),
        9,
        "Should create exactly 9 tables (5 nodes + 4 relationships)"
    );

    // Verify all are created
    for schema_result in &result {
        assert!(
            schema_result.created,
            "Table {} should be newly created",
            schema_result.relation
        );
    }
}

#[test]
fn test_setup_creates_all_tables() {
    let db = open_mem_db().expect("Failed to open database");
    let result = create_schema(db.as_ref()).expect("Failed to create schema");

    // Verify all expected tables
    let expected = relation_names();
    assert_eq!(result.len(), expected.len());

    for name in expected {
        assert!(
            result.iter().any(|r| r.relation == name),
            "Missing table: {}",
            name
        );
    }
}

#[test]
fn test_setup_creates_node_tables() {
    let db = open_mem_db().expect("Failed to open database");
    let result = create_schema(db.as_ref()).expect("Failed to create schema");

    let node_table_names = ["module", "function", "clause", "type", "field"];

    for name in &node_table_names {
        assert!(
            result.iter().any(|r| r.relation == *name),
            "Missing node table: {}",
            name
        );
    }
}

#[test]
fn test_setup_creates_relationship_tables() {
    let db = open_mem_db().expect("Failed to open database");
    let result = create_schema(db.as_ref()).expect("Failed to create schema");

    let rel_table_names = ["defines", "has_clause", "calls", "has_field"];

    for name in &rel_table_names {
        assert!(
            result.iter().any(|r| r.relation == *name),
            "Missing relationship table: {}",
            name
        );
    }
}

// ==================== Two-Phase Creation Order Tests ====================

#[test]
fn test_node_tables_created_first() {
    let db = open_mem_db().expect("Failed to open database");
    let result = create_schema(db.as_ref()).expect("Failed to create schema");

    // Verify creation order: first 5 should be nodes, last 4 should be relationships
    let node_tables = vec!["module", "function", "clause", "type", "field"];
    let rel_tables = vec!["defines", "has_clause", "calls", "has_field"];

    // Extract table names in order
    let table_names: Vec<_> = result.iter().map(|r| r.relation.as_str()).collect();

    // First 5 should be node tables
    for (i, table_name) in table_names.iter().enumerate().take(5) {
        assert!(
            node_tables.contains(table_name),
            "Position {} should be a node table, got {}",
            i,
            table_name
        );
    }

    // Last 4 should be relationship tables
    for (i, table_name) in table_names.iter().enumerate().skip(5) {
        assert!(
            rel_tables.contains(table_name),
            "Position {} should be a relationship table, got {}",
            i,
            table_name
        );
    }
}

// ==================== Idempotency Tests ====================

#[test]
fn test_setup_idempotency() {
    let db = open_mem_db().expect("Failed to open database");

    // First run - creates tables
    let result1 = create_schema(db.as_ref()).expect("Failed to create schema (first run)");
    assert_eq!(result1.len(), 9);
    assert!(
        result1.iter().all(|r| r.created),
        "All tables should be newly created on first run"
    );

    // Second run - should be idempotent
    let result2 = create_schema(db.as_ref()).expect("Failed to create schema (second run)");
    assert_eq!(result2.len(), 9);
    assert!(
        result2.iter().all(|r| !r.created),
        "All tables should already exist on second run"
    );
}

#[test]
fn test_setup_idempotency_multiple_runs() {
    let db = open_mem_db().expect("Failed to open database");

    // Run schema creation multiple times
    for run in 1..=3 {
        let result = create_schema(db.as_ref())
            .expect(&format!("Failed to create schema (run {})", run));
        assert_eq!(result.len(), 9, "Run {}: Should always have 9 tables", run);

        let expected_created = run == 1;
        for r in &result {
            assert_eq!(
                r.created, expected_created,
                "Run {}: {}.created should be {}",
                run, r.relation, expected_created
            );
        }
    }
}

// ==================== Query Execution Tests ====================

#[test]
fn test_execute_ddl_statement() {
    let db = open_mem_db().expect("Failed to open database");

    // Create schema first
    create_schema(db.as_ref()).expect("Failed to create schema");

    // Execute a simple DDL statement to verify database accepts queries
    let result = db.execute_query(
        "DEFINE TABLE test_table SCHEMAFULL;",
        QueryParams::new(),
    );

    assert!(result.is_ok(), "Should be able to execute DDL statements");
}

#[test]
fn test_query_with_parameters() {
    let db = open_mem_db().expect("Failed to open database");

    // Create schema
    create_schema(db.as_ref()).expect("Failed to create schema");

    // Create a simple DDL statement with parameters
    let params = QueryParams::new()
        .with_str("table_name", "test");

    let result = db.execute_query(
        "DEFINE TABLE params_test SCHEMAFULL; DEFINE FIELD name ON params_test TYPE string;",
        params,
    );

    assert!(
        result.is_ok(),
        "Should be able to execute queries with parameters"
    );
}

// ==================== Database Connection Tests ====================

#[test]
fn test_open_mem_returns_valid_database() {
    let db = open_mem_db().expect("Failed to open in-memory database");

    // Should be able to execute a basic DDL query
    let result = db.execute_query(
        "DEFINE TABLE check_db SCHEMAFULL;",
        QueryParams::new(),
    );

    assert!(result.is_ok(), "Should be able to execute basic query");
}

#[test]
fn test_open_persistent_database() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Should be able to open and use database
    let db = open_database(&db_path).expect("Failed to open persistent database");

    let result = db.execute_query(
        "DEFINE TABLE check_persistent SCHEMAFULL;",
        QueryParams::new(),
    );

    assert!(result.is_ok(), "Should be able to execute basic query");
}

// ==================== Multiple Databases Tests ====================

#[test]
fn test_multiple_in_memory_databases_are_independent() {
    let db1 = open_mem_db().expect("Failed to open database 1");
    let db2 = open_mem_db().expect("Failed to open database 2");

    // Create different schemas in each database
    let result1 = create_schema(db1.as_ref()).expect("Failed to create schema in db1");
    let result2 = create_schema(db2.as_ref()).expect("Failed to create schema in db2");

    // Both should have schema
    assert_eq!(result1.len(), 9);
    assert_eq!(result2.len(), 9);

    // Verify we can execute queries independently in each
    let query1 = db1.execute_query(
        "DEFINE TABLE db1_test SCHEMAFULL;",
        QueryParams::new(),
    );

    let query2 = db2.execute_query(
        "DEFINE TABLE db2_test SCHEMAFULL;",
        QueryParams::new(),
    );

    assert!(query1.is_ok());
    assert!(query2.is_ok());
}
