//! Shared test utilities for execute and integration tests.
//!
//! This module provides common helpers used across command execute tests.

use std::io::Write;

use tempfile::NamedTempFile;

use crate::commands::ImportCmd;
use crate::commands::Execute;

/// Create a temporary file containing the given content.
///
/// Used to create JSON files for importing test data.
pub fn create_temp_json_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write temp file");
    file
}

/// Create a temporary database file.
pub fn create_temp_db() -> NamedTempFile {
    NamedTempFile::new().expect("Failed to create temp db file")
}

/// Import JSON content into a database and return the db file.
///
/// This is the standard setup for execute tests: create a DB,
/// import test data, return the DB file for command execution.
pub fn setup_test_db(json_content: &str, project: &str) -> NamedTempFile {
    let db_file = create_temp_db();
    let json_file = create_temp_json_file(json_content);

    let import_cmd = ImportCmd {
        file: json_file.path().to_path_buf(),
        project: project.to_string(),
        clear: false,
    };
    import_cmd
        .execute(db_file.path())
        .expect("Import should succeed");

    db_file
}

/// Execute a command against a database and return the result.
///
/// Convenience wrapper that handles the path conversion.
pub fn execute_cmd<C: Execute>(cmd: C, db_file: &NamedTempFile) -> Result<C::Output, Box<dyn std::error::Error>> {
    cmd.execute(db_file.path())
}

/// Execute a command against an empty (uninitialized) database.
///
/// Used to verify commands fail gracefully on empty DBs.
pub fn execute_on_empty_db<C: Execute>(cmd: C) -> Result<C::Output, Box<dyn std::error::Error>> {
    let db_file = create_temp_db();
    cmd.execute(db_file.path())
}

// =============================================================================
// Fixture-based helpers
// =============================================================================

use crate::fixtures;

/// Create a test database with call graph data.
///
/// Use for: trace, reverse_trace, calls_from, calls_to, path, hotspots,
/// unused, depends_on, depended_by
pub fn call_graph_db(project: &str) -> NamedTempFile {
    setup_test_db(fixtures::CALL_GRAPH, project)
}

/// Create a test database with type signature data.
///
/// Use for: search (functions kind), function
pub fn type_signatures_db(project: &str) -> NamedTempFile {
    setup_test_db(fixtures::TYPE_SIGNATURES, project)
}

/// Create a test database with struct definitions.
///
/// Use for: struct command
pub fn structs_db(project: &str) -> NamedTempFile {
    setup_test_db(fixtures::STRUCTS, project)
}

// =============================================================================
// Output fixture helpers
// =============================================================================

use std::path::Path;

/// Load a fixture file from src/fixtures/output/<command>/<name>
pub fn load_output_fixture(command: &str, name: &str) -> String {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/fixtures/output")
        .join(command)
        .join(name);

    std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture_path.display(), e))
}
