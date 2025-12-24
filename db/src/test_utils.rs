//! Shared test utilities for database and integration tests.
//!
//! This module provides common helpers for setting up test databases with fixture data.

#[cfg(feature = "test-utils")]
use std::io::Write;

use crate::backend::Database;
#[cfg(feature = "test-utils")]
use tempfile::NamedTempFile;

#[cfg(any(test, feature = "test-utils"))]
use crate::queries::import::import_json_str;
use crate::db::{open_mem_db, get_cozo_instance};
use std::error::Error;

#[cfg(all(any(test, feature = "test-utils"), feature = "backend-cozo"))]
use cozo::DbInstance;

/// Create a temporary file containing the given content.
///
/// Used to create JSON files for importing test data.
#[cfg(feature = "test-utils")]
pub fn create_temp_json_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write temp file");
    file
}

/// Create an in-memory database and import JSON content.
///
/// This is the standard setup for execute tests: create an in-memory DB,
/// import test data, return the DB instance for command execution.
#[cfg(any(test, feature = "test-utils"))]
pub fn setup_test_db(json_content: &str, project: &str) -> Box<dyn Database> {
    let db = open_mem_db().expect("Failed to create in-memory DB");
    import_json_str(&*db, json_content, project).expect("Import should succeed");
    db
}

/// Create an empty in-memory database.
///
/// Used to verify queries fail gracefully on empty DBs.
#[cfg(any(test, feature = "test-utils"))]
pub fn setup_empty_test_db() -> Box<dyn Database> {
    open_mem_db().expect("Failed to create in-memory DB")
}

// =============================================================================
// Fixture-based helpers
// =============================================================================

#[cfg(any(test, feature = "test-utils"))]
use crate::fixtures;

/// Create a test database with call graph data.
///
/// Use for: trace, reverse_trace, calls_from, calls_to, path, hotspots,
/// unused, depends_on, depended_by
#[cfg(any(test, feature = "test-utils"))]
pub fn call_graph_db(project: &str) -> Box<dyn Database> {
    setup_test_db(fixtures::CALL_GRAPH, project)
}

/// Create a test database with type signature data.
///
/// Use for: search (functions kind), function
#[cfg(any(test, feature = "test-utils"))]
pub fn type_signatures_db(project: &str) -> Box<dyn Database> {
    setup_test_db(fixtures::TYPE_SIGNATURES, project)
}

/// Create a test database with struct definitions.
///
/// Use for: struct command
#[cfg(any(test, feature = "test-utils"))]
pub fn structs_db(project: &str) -> Box<dyn Database> {
    setup_test_db(fixtures::STRUCTS, project)
}

/// Helper to extract DbInstance from Box<dyn Database> for test compatibility.
///
/// Use this in tests when you need to pass a &DbInstance to query functions.
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-cozo"))]
pub fn get_db_instance(db: &Box<dyn Database>) -> &DbInstance {
    get_cozo_instance(&**db)
}

// =============================================================================
// Output fixture helpers
// =============================================================================

use std::path::Path;

/// Load a fixture file from src/fixtures/output/<command>/<name>
#[cfg(any(test, feature = "test-utils"))]
pub fn load_output_fixture(command: &str, name: &str) -> String {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/fixtures/output")
        .join(command)
        .join(name);

    std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path.display(), e))
}
