//! Shared test utilities for database and integration tests.
//!
//! This module provides common helpers for setting up test databases with fixture data.

#[cfg(feature = "test-utils")]
use std::io::Write;

use crate::backend::Database;
#[cfg(feature = "test-utils")]
use tempfile::NamedTempFile;

use crate::db::open_mem_db;
#[cfg(any(test, feature = "test-utils"))]
use crate::queries::import::import_json_str;

#[cfg(all(any(test, feature = "test-utils"), feature = "backend-cozo"))]
use crate::db::get_cozo_instance;

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

// =============================================================================
// SurrealDB Test Fixture Infrastructure
// =============================================================================

#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
use crate::backend::QueryParams;

#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
use crate::queries::schema;

#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
use std::error::Error;

/// Insert a module node directly into the database.
///
/// Creates a new module record with the given name. Module names are unique
/// and serve as the primary key for module nodes.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `name` - The module name (must be unique)
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the module already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_module(db: &dyn Database, name: &str) -> Result<(), Box<dyn Error>> {
    let query = "CREATE modules:[$name] SET name = $name, file = \"\", source = \"unknown\";";
    let params = QueryParams::new().with_str("name", name);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a function node directly into the database.
///
/// Creates a new function record with signature (module_name, name, arity).
/// Functions are derived from function_locations and represent unique function
/// identities regardless of clause count.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module containing this function
/// * `name` - The function name
/// * `arity` - The function arity (number of parameters)
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the function already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_function(
    db: &dyn Database,
    module_name: &str,
    name: &str,
    arity: i64,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        CREATE functions:[$module_name, $name, $arity] SET
            module_name = $module_name,
            name = $name,
            arity = $arity;
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("name", name)
        .with_int("arity", arity);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a clause node directly into the database.
///
/// Creates a new clause record representing a function clause (pattern-matched head).
/// The clause natural key is (module_name, function_name, arity, line) and must be unique.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module containing this clause
/// * `function_name` - The name of the function this clause belongs to
/// * `arity` - The arity of the function
/// * `line` - The line number where this clause is defined
/// * `source_file` - The source file path (relative)
/// * `kind` - The function kind (def, defp, defmacro, etc.)
/// * `complexity` - Code complexity metric for this clause
/// * `depth` - Max nesting depth metric for this clause
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the clause already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_clause(
    db: &dyn Database,
    module_name: &str,
    function_name: &str,
    arity: i64,
    line: i64,
    source_file: &str,
    kind: &str,
    complexity: i64,
    depth: i64,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        CREATE clauses:[$module_name, $function_name, $arity, $line] SET
            module_name = $module_name,
            function_name = $function_name,
            arity = $arity,
            line = $line,
            source_file = $source_file,
            source_file_absolute = "",
            kind = $kind,
            start_line = $line,
            end_line = $line,
            pattern = "",
            guard = NONE,
            source_sha = "",
            ast_sha = "",
            complexity = $complexity,
            max_nesting_depth = $depth,
            generated_by = NONE,
            macro_source = NONE;
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("function_name", function_name)
        .with_int("arity", arity)
        .with_int("line", line)
        .with_str("source_file", source_file)
        .with_str("kind", kind)
        .with_int("complexity", complexity)
        .with_int("depth", depth);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a clause node with hash values for duplicate detection tests.
///
/// Creates a new clause record representing a function clause (pattern-matched head).
/// This variant is used for testing duplicate detection queries and includes hash fields.
/// The clause natural key is (module_name, function_name, arity, line) and must be unique.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module containing this clause
/// * `function_name` - The name of the function this clause belongs to
/// * `arity` - The arity of the function
/// * `line` - The line number where this clause is defined
/// * `source_file` - The source file path (relative)
/// * `kind` - The function kind (def, defp, defmacro, etc.)
/// * `complexity` - Code complexity metric for this clause
/// * `depth` - Max nesting depth metric for this clause
/// * `source_sha` - SHA hash of the source code (for exact duplicates)
/// * `ast_sha` - SHA hash of the AST (for structural duplicates)
/// * `generated_by` - Optional: name of tool that generated this (e.g., "phoenix")
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the clause already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_clause_with_hash(
    db: &dyn Database,
    module_name: &str,
    function_name: &str,
    arity: i64,
    line: i64,
    source_file: &str,
    kind: &str,
    complexity: i64,
    depth: i64,
    source_sha: &str,
    ast_sha: &str,
    generated_by: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    // Build the generated_by value based on whether it's provided
    let generated_by_value = if let Some(generated) = generated_by {
        format!("\"{}\"", generated)
    } else {
        "NONE".to_string()
    };

    let query = format!(
        r#"
        CREATE clauses:[$module_name, $function_name, $arity, $line] SET
            module_name = $module_name,
            function_name = $function_name,
            arity = $arity,
            line = $line,
            source_file = $source_file,
            source_file_absolute = "",
            kind = $kind,
            start_line = $line,
            end_line = $line,
            pattern = "",
            guard = NONE,
            source_sha = $source_sha,
            ast_sha = $ast_sha,
            complexity = $complexity,
            max_nesting_depth = $depth,
            generated_by = {},
            macro_source = NONE;
        "#,
        generated_by_value
    );

    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("function_name", function_name)
        .with_int("arity", arity)
        .with_int("line", line)
        .with_str("source_file", source_file)
        .with_str("kind", kind)
        .with_int("complexity", complexity)
        .with_int("depth", depth)
        .with_str("source_sha", source_sha)
        .with_str("ast_sha", ast_sha);

    db.execute_query(&query, params)?;
    Ok(())
}

/// Insert a type node directly into the database.
///
/// Creates a new type/struct definition record. The type natural key is
/// (module_name, name) and must be unique within the database.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module containing this type
/// * `name` - The type name
/// * `kind` - The type kind (e.g., "struct", "enum", "record")
/// * `definition` - The type definition or signature
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the type already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_type(
    db: &dyn Database,
    module_name: &str,
    name: &str,
    kind: &str,
    definition: &str,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        CREATE types:[$module_name, $name] SET
            module_name = $module_name,
            name = $name,
            kind = $kind,
            params = "",
            line = 1,
            definition = $definition;
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("name", name)
        .with_str("kind", kind)
        .with_str("definition", definition);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a spec node directly into the database.
///
/// Creates a new spec record representing a @spec or @callback definition.
/// The spec natural key is (module_name, function_name, arity, clause_index).
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module containing this spec
/// * `function_name` - The function this spec describes
/// * `arity` - The arity of the function
/// * `kind` - The spec kind ("spec" or "callback")
/// * `line` - The line number where the spec is defined
/// * `clause_index` - Index for multi-clause specs (0 for single clause)
/// * `full` - The full spec string (e.g., "@spec foo(integer()) :: atom()")
/// * `input_strings` - Array of input type strings (e.g., ["integer()", "keyword()"])
/// * `return_strings` - Array of return type strings (e.g., ["atom()"])
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the spec already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_spec(
    db: &dyn Database,
    module_name: &str,
    function_name: &str,
    arity: i64,
    kind: &str,
    line: i64,
    clause_index: i64,
    full: &str,
    input_strings: &[&str],
    return_strings: &[&str],
) -> Result<(), Box<dyn Error>> {
    // Convert input strings to SurrealQL array format
    let inputs_array = format!(
        "[{}]",
        input_strings
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let returns_array = format!(
        "[{}]",
        return_strings
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let query = format!(
        r#"
        CREATE specs:[$module_name, $function_name, $arity, $clause_index] SET
            module_name = $module_name,
            function_name = $function_name,
            arity = $arity,
            kind = $kind,
            line = $line,
            clause_index = $clause_index,
            input_strings = {},
            return_strings = {},
            full = $full;
    "#,
        inputs_array, returns_array
    );
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("function_name", function_name)
        .with_int("arity", arity)
        .with_str("kind", kind)
        .with_int("line", line)
        .with_int("clause_index", clause_index)
        .with_str("full", full);
    db.execute_query(&query, params)?;
    Ok(())
}

/// Insert a field node directly into the database.
///
/// Creates a new struct field record. The field natural key is
/// (module_name, name). In Elixir, struct name equals module name.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module that defines the struct
/// * `field_name` - The field name
/// * `default_value` - The default value for this field (as string)
/// * `required` - Whether the field is required (enforced_keys)
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the field already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_field(
    db: &dyn Database,
    module_name: &str,
    field_name: &str,
    default_value: &str,
    required: bool,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        CREATE fields:[$module_name, $field_name] SET
            module_name = $module_name,
            name = $field_name,
            default_value = $default_value,
            required = $required;
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("field_name", field_name)
        .with_str("default_value", default_value)
        .with_bool("required", required);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a call relationship edge between two functions.
///
/// Creates a directed edge from caller function to callee function, recording
/// the call type (local or remote), source file, and line number.
/// The caller_clause_id is constructed from the caller function's clause at the given line.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `from_module` - Module containing the caller function
/// * `from_fn` - Name of the caller function
/// * `from_arity` - Arity of the caller function
/// * `to_module` - Module containing the callee function
/// * `to_fn` - Name of the callee function
/// * `to_arity` - Arity of the callee function
/// * `call_type` - Type of call: "local" or "remote"
/// * `caller_kind` - Kind of the caller function (def, defp, defmacro, etc.)
/// * `file` - Source file where the call occurs
/// * `line` - Line number where the call occurs
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the relationship cannot be created or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_call(
    db: &dyn Database,
    from_module: &str,
    from_fn: &str,
    from_arity: i64,
    to_module: &str,
    to_fn: &str,
    to_arity: i64,
    call_type: &str,
    caller_kind: &str,
    file: &str,
    line: i64,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        RELATE
            functions:[$from_module, $from_fn, $from_arity]
            ->calls->
            functions:[$to_module, $to_fn, $to_arity]
        SET
            call_type = $call_type,
            caller_kind = $caller_kind,
            file = $file,
            line = $line,
            caller_clause_id = clauses:[$from_module, $from_fn, $from_arity, $line];
    "#;
    let params = QueryParams::new()
        .with_str("from_module", from_module)
        .with_str("from_fn", from_fn)
        .with_int("from_arity", from_arity)
        .with_str("to_module", to_module)
        .with_str("to_fn", to_fn)
        .with_int("to_arity", to_arity)
        .with_str("call_type", call_type)
        .with_str("caller_kind", caller_kind)
        .with_str("file", file)
        .with_int("line", line);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a defines relationship edge from module to entity.
///
/// Creates an edge representing module containment: module defines a function or type.
/// This relationship is used for traversing what entities a module contains.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module that defines the entity
/// * `entity_type` - The entity type: "functions" or "types"
/// * `entity_id` - The record ID of the entity (e.g., "module:name:arity" for function)
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the relationship cannot be created or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
#[allow(dead_code)] // Helper for future tests
fn insert_defines(
    db: &dyn Database,
    module_name: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), Box<dyn Error>> {
    let query = format!(
        "RELATE modules:⟨$module_name⟩ ->defines-> {}:⟨$entity_id⟩;",
        entity_type
    );
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("entity_id", entity_id);
    db.execute_query(&query, params)?;
    Ok(())
}

/// Insert a has_clause relationship edge from function to clause.
///
/// Creates an edge linking a function to one of its individual clauses
/// (pattern-matched heads). This relationship is essential for understanding
/// the structure of pattern-matched functions.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `function_id` - The function record ID in format "module:name:arity"
/// * `clause_id` - The clause record ID
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the relationship cannot be created or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_has_clause(
    db: &dyn Database,
    module_name: &str,
    function_name: &str,
    arity: i64,
    line: i64,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        RELATE functions:[$module_name, $function_name, $arity]
        ->has_clause->
        clauses:[$module_name, $function_name, $arity, $line];
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("function_name", function_name)
        .with_int("arity", arity)
        .with_int("line", line);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a has_field relationship edge from module to field.
///
/// Creates an edge linking a module (that defines a struct) to one of its fields.
/// In Elixir, struct name equals module name, so fields belong to modules.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module that defines the struct
/// * `field_name` - Name of the field
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the relationship cannot be created or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_has_field(
    db: &dyn Database,
    module_name: &str,
    field_name: &str,
) -> Result<(), Box<dyn Error>> {
    let query = "RELATE modules:[$module_name] ->has_field-> fields:[$module_name, $field_name];";
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("field_name", field_name);
    db.execute_query(query, params)?;
    Ok(())
}

/// Create a test database with call graph data (simple version).
///
/// Sets up an in-memory SurrealDB instance with the complete graph schema
/// and minimal fixtures containing:
/// - Two modules (module_a, module_b)
/// - Three functions (foo/1, bar/2 in module_a, baz/0 in module_b)
/// - Two call relationships (foo calls bar locally, foo calls baz remotely)
///
/// This fixture is suitable for basic testing of:
/// - Trace queries (following call chains)
/// - Reverse trace queries (finding callers)
/// - Path finding between functions
/// - Call graph analysis
///
/// For more realistic, complex testing, use `surreal_call_graph_db_complex()`.
///
/// # Returns
/// A boxed trait object containing the configured database instance
///
/// # Panics
/// Panics if database creation or schema setup fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
pub fn surreal_call_graph_db() -> Box<dyn Database> {
    let db = open_mem_db().expect("Failed to create in-memory database");
    schema::create_schema(&*db).expect("Failed to create schema");

    insert_module(&*db, "module_a").expect("Failed to insert module_a");
    insert_module(&*db, "module_b").expect("Failed to insert module_b");

    insert_function(&*db, "module_a", "foo", 1)
        .expect("Failed to insert foo/1");
    insert_function(&*db, "module_a", "bar", 2)
        .expect("Failed to insert bar/2");
    insert_function(&*db, "module_b", "baz", 0)
        .expect("Failed to insert baz/0");

    // Create clauses for each function (required for call relationships)
    // Clause lines must match the lines where calls occur
    insert_clause(&*db, "module_a", "foo", 1, 10, "lib/module_a.ex", "def", 1, 1)
        .expect("Failed to insert clause for foo/1 at line 10");
    insert_clause(&*db, "module_a", "bar", 2, 8, "lib/module_a.ex", "defp", 2, 1)
        .expect("Failed to insert clause for bar/2 at line 8");
    insert_clause(&*db, "module_b", "baz", 0, 3, "lib/module_b.ex", "def", 1, 1)
        .expect("Failed to insert clause for baz/0 at line 3");

    // Create calls - line numbers must match the caller's clause line
    insert_call(
        &*db, "module_a", "foo", 1, "module_a", "bar", 2,
        "local", "def", "lib/module_a.ex", 10,
    )
    .expect("Failed to insert call: foo -> bar");

    // Second call from foo - need another clause at line 15
    insert_clause(&*db, "module_a", "foo", 1, 15, "lib/module_a.ex", "def", 1, 1)
        .expect("Failed to insert clause for foo/1 at line 15");

    insert_call(
        &*db, "module_a", "foo", 1, "module_b", "baz", 0,
        "remote", "def", "lib/module_a.ex", 15,
    )
    .expect("Failed to insert call: foo -> baz");

    db
}

/// Create a test database with complex call graph data (modeled after call_graph.json fixture).
///
/// Sets up an in-memory SurrealDB instance with realistic test data containing:
/// - 5 modules: MyApp.Controller, MyApp.Accounts, MyApp.Service, MyApp.Repo, MyApp.Notifier
/// - 15 functions with various arities (0-2) and kinds (def/defp)
/// - Multiple clauses per function showing pattern matching
/// - 11 call edges forming a realistic call graph
/// - Realistic file paths, line numbers, and patterns
///
/// This fixture models a realistic web application architecture:
/// - Controller layer (public API endpoints)
/// - Business logic layer (Accounts, Service)
/// - Data access layer (Repo)
/// - External services (Notifier)
///
/// Suitable for comprehensive testing of:
/// - Complex trace queries across multiple layers
/// - Reverse trace queries (finding all callers)
/// - Path finding between distant functions
/// - Hotspot analysis (most-called functions)
/// - Dependency analysis (module relationships)
/// - Unused function detection
///
/// # Returns
/// A boxed trait object containing the configured database instance
///
/// # Panics
/// Panics if database creation or schema setup fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
pub fn surreal_call_graph_db_complex() -> Box<dyn Database> {
    let db = open_mem_db().expect("Failed to create in-memory database");
    schema::create_schema(&*db).expect("Failed to create schema");

    // Create modules matching call_graph.json
    insert_module(&*db, "MyApp.Controller").expect("Failed to insert MyApp.Controller");
    insert_module(&*db, "MyApp.Accounts").expect("Failed to insert MyApp.Accounts");
    insert_module(&*db, "MyApp.Service").expect("Failed to insert MyApp.Service");
    insert_module(&*db, "MyApp.Repo").expect("Failed to insert MyApp.Repo");
    insert_module(&*db, "MyApp.Notifier").expect("Failed to insert MyApp.Notifier");

    // Additional modules for cycle testing
    insert_module(&*db, "MyApp.Logger").expect("Failed to insert MyApp.Logger");
    insert_module(&*db, "MyApp.Events").expect("Failed to insert MyApp.Events");
    insert_module(&*db, "MyApp.Cache").expect("Failed to insert MyApp.Cache");
    insert_module(&*db, "MyApp.Metrics").expect("Failed to insert MyApp.Metrics");

    // Controller functions (public API)
    insert_function(&*db, "MyApp.Controller", "index", 2)
        .expect("Failed to insert index/2");
    insert_function(&*db, "MyApp.Controller", "show", 2)
        .expect("Failed to insert show/2");
    insert_function(&*db, "MyApp.Controller", "create", 2)
        .expect("Failed to insert create/2");

    // Accounts functions (business logic)
    insert_function(&*db, "MyApp.Accounts", "get_user", 1)
        .expect("Failed to insert get_user/1");
    insert_function(&*db, "MyApp.Accounts", "get_user", 2)
        .expect("Failed to insert get_user/2");
    insert_function(&*db, "MyApp.Accounts", "list_users", 0)
        .expect("Failed to insert list_users/0");
    insert_function(&*db, "MyApp.Accounts", "validate_email", 1)
        .expect("Failed to insert validate_email/1");

    // Service functions
    insert_function(&*db, "MyApp.Service", "process_request", 2)
        .expect("Failed to insert process_request/2");
    insert_function(&*db, "MyApp.Service", "transform_data", 1)
        .expect("Failed to insert transform_data/1");

    // Repo functions (data access)
    insert_function(&*db, "MyApp.Repo", "get", 2)
        .expect("Failed to insert get/2");
    insert_function(&*db, "MyApp.Repo", "all", 1)
        .expect("Failed to insert all/1");
    insert_function(&*db, "MyApp.Repo", "insert", 1)
        .expect("Failed to insert insert/1");
    insert_function(&*db, "MyApp.Repo", "query", 2)
        .expect("Failed to insert query/2");

    // Notifier functions
    insert_function(&*db, "MyApp.Notifier", "send_email", 2)
        .expect("Failed to insert send_email/2");
    insert_function(&*db, "MyApp.Notifier", "format_message", 1)
        .expect("Failed to insert format_message/1");
    insert_function(&*db, "MyApp.Notifier", "on_cache_update", 1)
        .expect("Failed to insert on_cache_update/1");

    // Controller - additional function for cycle B
    insert_function(&*db, "MyApp.Controller", "handle_event", 1)
        .expect("Failed to insert handle_event/1");

    // Accounts - additional function for cycle B
    insert_function(&*db, "MyApp.Accounts", "notify_change", 1)
        .expect("Failed to insert notify_change/1");

    // Service - additional function for cycle A
    insert_function(&*db, "MyApp.Service", "get_context", 1)
        .expect("Failed to insert get_context/1");

    // Logger functions (for cycles A and C)
    insert_function(&*db, "MyApp.Logger", "log_query", 2)
        .expect("Failed to insert log_query/2");
    insert_function(&*db, "MyApp.Logger", "log_metric", 1)
        .expect("Failed to insert log_metric/1");
    insert_function(&*db, "MyApp.Logger", "debug", 1)
        .expect("Failed to insert debug/1");

    // Events functions (for cycles B and C)
    insert_function(&*db, "MyApp.Events", "publish", 2)
        .expect("Failed to insert publish/2");
    insert_function(&*db, "MyApp.Events", "emit", 2)
        .expect("Failed to insert emit/2");
    insert_function(&*db, "MyApp.Events", "subscribe", 2)
        .expect("Failed to insert subscribe/2");

    // Cache functions (for cycles B and C)
    insert_function(&*db, "MyApp.Cache", "invalidate", 1)
        .expect("Failed to insert invalidate/1");
    insert_function(&*db, "MyApp.Cache", "store", 2)
        .expect("Failed to insert store/2");
    insert_function(&*db, "MyApp.Cache", "fetch", 1)
        .expect("Failed to insert fetch/1");

    // Metrics functions (for cycle C)
    insert_function(&*db, "MyApp.Metrics", "record", 2)
        .expect("Failed to insert record/2");
    insert_function(&*db, "MyApp.Metrics", "increment", 1)
        .expect("Failed to insert increment/1");

    // Create clauses with realistic line numbers and file paths
    // Controller.index/2 - calls Accounts.list_users/0
    insert_clause(&*db, "MyApp.Controller", "index", 2, 5, "lib/my_app/controller.ex", "def", 3, 2)
        .expect("Failed to insert clause for Controller.index/2");
    insert_has_clause(&*db, "MyApp.Controller", "index", 2, 5)
        .expect("Failed to insert has_clause for Controller.index/2 at line 5");
    insert_clause(&*db, "MyApp.Controller", "index", 2, 7, "lib/my_app/controller.ex", "def", 1, 1)
        .expect("Failed to insert clause for Controller.index/2 at line 7");
    insert_has_clause(&*db, "MyApp.Controller", "index", 2, 7)
        .expect("Failed to insert has_clause for Controller.index/2 at line 7");

    // Controller.show/2 - calls Accounts.get_user/2
    insert_clause(&*db, "MyApp.Controller", "show", 2, 12, "lib/my_app/controller.ex", "def", 3, 2)
        .expect("Failed to insert clause for Controller.show/2");
    insert_has_clause(&*db, "MyApp.Controller", "show", 2, 12)
        .expect("Failed to insert has_clause for Controller.show/2 at line 12");
    insert_clause(&*db, "MyApp.Controller", "show", 2, 15, "lib/my_app/controller.ex", "def", 1, 1)
        .expect("Failed to insert clause for Controller.show/2 at line 15");
    insert_has_clause(&*db, "MyApp.Controller", "show", 2, 15)
        .expect("Failed to insert has_clause for Controller.show/2 at line 15");

    // Controller.create/2 - calls Service.process_request/2
    insert_clause(&*db, "MyApp.Controller", "create", 2, 20, "lib/my_app/controller.ex", "def", 5, 3)
        .expect("Failed to insert clause for Controller.create/2");
    insert_has_clause(&*db, "MyApp.Controller", "create", 2, 20)
        .expect("Failed to insert has_clause for Controller.create/2 at line 20");
    insert_clause(&*db, "MyApp.Controller", "create", 2, 25, "lib/my_app/controller.ex", "def", 2, 2)
        .expect("Failed to insert clause for Controller.create/2 at line 25");
    insert_has_clause(&*db, "MyApp.Controller", "create", 2, 25)
        .expect("Failed to insert has_clause for Controller.create/2 at line 25");

    // Accounts.get_user/1 - calls Repo.get/2
    insert_clause(&*db, "MyApp.Accounts", "get_user", 1, 10, "lib/my_app/accounts.ex", "def", 2, 1)
        .expect("Failed to insert clause for Accounts.get_user/1");
    insert_has_clause(&*db, "MyApp.Accounts", "get_user", 1, 10)
        .expect("Failed to insert has_clause for Accounts.get_user/1 at line 10");
    insert_clause(&*db, "MyApp.Accounts", "get_user", 1, 12, "lib/my_app/accounts.ex", "def", 1, 1)
        .expect("Failed to insert clause for Accounts.get_user/1 at line 12");
    insert_has_clause(&*db, "MyApp.Accounts", "get_user", 1, 12)
        .expect("Failed to insert has_clause for Accounts.get_user/1 at line 12");

    // Accounts.get_user/2 - calls get_user/1
    insert_clause(&*db, "MyApp.Accounts", "get_user", 2, 17, "lib/my_app/accounts.ex", "def", 2, 1)
        .expect("Failed to insert clause for Accounts.get_user/2");
    insert_has_clause(&*db, "MyApp.Accounts", "get_user", 2, 17)
        .expect("Failed to insert has_clause for Accounts.get_user/2 at line 17");

    // Accounts.list_users/0 - calls Repo.all/1
    insert_clause(&*db, "MyApp.Accounts", "list_users", 0, 24, "lib/my_app/accounts.ex", "def", 2, 1)
        .expect("Failed to insert clause for Accounts.list_users/0");
    insert_has_clause(&*db, "MyApp.Accounts", "list_users", 0, 24)
        .expect("Failed to insert has_clause for Accounts.list_users/0 at line 24");

    // Accounts.validate_email/1
    insert_clause(&*db, "MyApp.Accounts", "validate_email", 1, 30, "lib/my_app/accounts.ex", "defp", 4, 2)
        .expect("Failed to insert clause for Accounts.validate_email/1");
    insert_has_clause(&*db, "MyApp.Accounts", "validate_email", 1, 30)
        .expect("Failed to insert has_clause for Accounts.validate_email/1 at line 30");

    // Accounts.__struct__/0 - compiler-generated function (for testing exclude_generated)
    insert_function(&*db, "MyApp.Accounts", "__struct__", 0)
        .expect("Failed to insert __struct__/0");
    insert_clause(&*db, "MyApp.Accounts", "__struct__", 0, 1, "lib/my_app/accounts.ex", "def", 1, 1)
        .expect("Failed to insert clause for Accounts.__struct__/0");
    insert_has_clause(&*db, "MyApp.Accounts", "__struct__", 0, 1)
        .expect("Failed to insert has_clause for Accounts.__struct__/0 at line 1");

    // Service.process_request/2 - calls Accounts.get_user/1 and Notifier.send_email/2
    insert_clause(&*db, "MyApp.Service", "process_request", 2, 8, "lib/my_app/service.ex", "def", 5, 3)
        .expect("Failed to insert clause for Service.process_request/2");
    insert_has_clause(&*db, "MyApp.Service", "process_request", 2, 8)
        .expect("Failed to insert has_clause for Service.process_request/2 at line 8");
    insert_clause(&*db, "MyApp.Service", "process_request", 2, 12, "lib/my_app/service.ex", "def", 2, 2)
        .expect("Failed to insert clause for Service.process_request/2 at line 12");
    insert_has_clause(&*db, "MyApp.Service", "process_request", 2, 12)
        .expect("Failed to insert has_clause for Service.process_request/2 at line 12");
    insert_clause(&*db, "MyApp.Service", "process_request", 2, 16, "lib/my_app/service.ex", "def", 1, 1)
        .expect("Failed to insert clause for Service.process_request/2 at line 16");
    insert_has_clause(&*db, "MyApp.Service", "process_request", 2, 16)
        .expect("Failed to insert has_clause for Service.process_request/2 at line 16");

    // Service.transform_data/1
    insert_clause(&*db, "MyApp.Service", "transform_data", 1, 22, "lib/my_app/service.ex", "defp", 3, 2)
        .expect("Failed to insert clause for Service.transform_data/1");
    insert_has_clause(&*db, "MyApp.Service", "transform_data", 1, 22)
        .expect("Failed to insert has_clause for Service.transform_data/1 at line 22");

    // Repo functions
    insert_clause(&*db, "MyApp.Repo", "get", 2, 10, "lib/my_app/repo.ex", "def", 2, 1)
        .expect("Failed to insert clause for Repo.get/2");
    insert_has_clause(&*db, "MyApp.Repo", "get", 2, 10)
        .expect("Failed to insert has_clause for Repo.get/2 at line 10");
    insert_clause(&*db, "MyApp.Repo", "all", 1, 15, "lib/my_app/repo.ex", "def", 2, 1)
        .expect("Failed to insert clause for Repo.all/1");
    insert_has_clause(&*db, "MyApp.Repo", "all", 1, 15)
        .expect("Failed to insert has_clause for Repo.all/1 at line 15");
    insert_clause(&*db, "MyApp.Repo", "insert", 1, 20, "lib/my_app/repo.ex", "def", 3, 2)
        .expect("Failed to insert clause for Repo.insert/1");
    insert_has_clause(&*db, "MyApp.Repo", "insert", 1, 20)
        .expect("Failed to insert has_clause for Repo.insert/1 at line 20");
    insert_clause(&*db, "MyApp.Repo", "query", 2, 28, "lib/my_app/repo.ex", "defp", 4, 2)
        .expect("Failed to insert clause for Repo.query/2");
    insert_has_clause(&*db, "MyApp.Repo", "query", 2, 28)
        .expect("Failed to insert has_clause for Repo.query/2 at line 28");

    // Notifier functions
    insert_clause(&*db, "MyApp.Notifier", "send_email", 2, 6, "lib/my_app/notifier.ex", "def", 3, 2)
        .expect("Failed to insert clause for Notifier.send_email/2");
    insert_has_clause(&*db, "MyApp.Notifier", "send_email", 2, 6)
        .expect("Failed to insert has_clause for Notifier.send_email/2 at line 6");
    insert_clause(&*db, "MyApp.Notifier", "format_message", 1, 15, "lib/my_app/notifier.ex", "defp", 2, 1)
        .expect("Failed to insert clause for Notifier.format_message/1");
    insert_has_clause(&*db, "MyApp.Notifier", "format_message", 1, 15)
        .expect("Failed to insert has_clause for Notifier.format_message/1 at line 15");
    insert_clause(&*db, "MyApp.Notifier", "on_cache_update", 1, 22, "lib/my_app/notifier.ex", "def", 2, 1)
        .expect("Failed to insert clause for Notifier.on_cache_update/1");
    insert_has_clause(&*db, "MyApp.Notifier", "on_cache_update", 1, 22)
        .expect("Failed to insert has_clause for Notifier.on_cache_update/1 at line 22");

    // Controller.handle_event/1 - for cycle B
    insert_clause(&*db, "MyApp.Controller", "handle_event", 1, 35, "lib/my_app/controller.ex", "def", 2, 1)
        .expect("Failed to insert clause for Controller.handle_event/1");
    insert_has_clause(&*db, "MyApp.Controller", "handle_event", 1, 35)
        .expect("Failed to insert has_clause for Controller.handle_event/1 at line 35");

    // Accounts.notify_change/1 - for cycle B
    insert_clause(&*db, "MyApp.Accounts", "notify_change", 1, 40, "lib/my_app/accounts.ex", "def", 2, 1)
        .expect("Failed to insert clause for Accounts.notify_change/1");
    insert_has_clause(&*db, "MyApp.Accounts", "notify_change", 1, 40)
        .expect("Failed to insert has_clause for Accounts.notify_change/1 at line 40");

    // Service.get_context/1 - for cycle A
    insert_clause(&*db, "MyApp.Service", "get_context", 1, 28, "lib/my_app/service.ex", "def", 1, 1)
        .expect("Failed to insert clause for Service.get_context/1");
    insert_has_clause(&*db, "MyApp.Service", "get_context", 1, 28)
        .expect("Failed to insert has_clause for Service.get_context/1 at line 28");

    // Logger functions
    insert_clause(&*db, "MyApp.Logger", "log_query", 2, 5, "lib/my_app/logger.ex", "def", 3, 2)
        .expect("Failed to insert clause for Logger.log_query/2");
    insert_has_clause(&*db, "MyApp.Logger", "log_query", 2, 5)
        .expect("Failed to insert has_clause for Logger.log_query/2 at line 5");
    insert_clause(&*db, "MyApp.Logger", "log_metric", 1, 12, "lib/my_app/logger.ex", "def", 2, 1)
        .expect("Failed to insert clause for Logger.log_metric/1");
    insert_has_clause(&*db, "MyApp.Logger", "log_metric", 1, 12)
        .expect("Failed to insert has_clause for Logger.log_metric/1 at line 12");
    insert_clause(&*db, "MyApp.Logger", "debug", 1, 18, "lib/my_app/logger.ex", "defp", 1, 1)
        .expect("Failed to insert clause for Logger.debug/1");
    insert_has_clause(&*db, "MyApp.Logger", "debug", 1, 18)
        .expect("Failed to insert has_clause for Logger.debug/1 at line 18");

    // Events functions
    insert_clause(&*db, "MyApp.Events", "publish", 2, 5, "lib/my_app/events.ex", "def", 3, 2)
        .expect("Failed to insert clause for Events.publish/2");
    insert_has_clause(&*db, "MyApp.Events", "publish", 2, 5)
        .expect("Failed to insert has_clause for Events.publish/2 at line 5");
    insert_clause(&*db, "MyApp.Events", "emit", 2, 12, "lib/my_app/events.ex", "def", 2, 1)
        .expect("Failed to insert clause for Events.emit/2");
    insert_has_clause(&*db, "MyApp.Events", "emit", 2, 12)
        .expect("Failed to insert has_clause for Events.emit/2 at line 12");
    insert_clause(&*db, "MyApp.Events", "subscribe", 2, 18, "lib/my_app/events.ex", "def", 2, 1)
        .expect("Failed to insert clause for Events.subscribe/2");
    insert_has_clause(&*db, "MyApp.Events", "subscribe", 2, 18)
        .expect("Failed to insert has_clause for Events.subscribe/2 at line 18");

    // Cache functions
    insert_clause(&*db, "MyApp.Cache", "invalidate", 1, 5, "lib/my_app/cache.ex", "def", 2, 1)
        .expect("Failed to insert clause for Cache.invalidate/1");
    insert_has_clause(&*db, "MyApp.Cache", "invalidate", 1, 5)
        .expect("Failed to insert has_clause for Cache.invalidate/1 at line 5");
    insert_clause(&*db, "MyApp.Cache", "store", 2, 10, "lib/my_app/cache.ex", "def", 2, 1)
        .expect("Failed to insert clause for Cache.store/2");
    insert_has_clause(&*db, "MyApp.Cache", "store", 2, 10)
        .expect("Failed to insert has_clause for Cache.store/2 at line 10");
    insert_clause(&*db, "MyApp.Cache", "fetch", 1, 16, "lib/my_app/cache.ex", "def", 2, 1)
        .expect("Failed to insert clause for Cache.fetch/1");
    insert_has_clause(&*db, "MyApp.Cache", "fetch", 1, 16)
        .expect("Failed to insert has_clause for Cache.fetch/1 at line 16");

    // Metrics functions
    insert_clause(&*db, "MyApp.Metrics", "record", 2, 5, "lib/my_app/metrics.ex", "def", 2, 1)
        .expect("Failed to insert clause for Metrics.record/2");
    insert_has_clause(&*db, "MyApp.Metrics", "record", 2, 5)
        .expect("Failed to insert has_clause for Metrics.record/2 at line 5");
    insert_clause(&*db, "MyApp.Metrics", "increment", 1, 12, "lib/my_app/metrics.ex", "def", 1, 1)
        .expect("Failed to insert clause for Metrics.increment/1");
    insert_has_clause(&*db, "MyApp.Metrics", "increment", 1, 12)
        .expect("Failed to insert has_clause for Metrics.increment/1 at line 12");

    // Create call relationships

    // Controller -> Accounts
    insert_call(
        &*db,
        "MyApp.Controller", "index", 2,
        "MyApp.Accounts", "list_users", 0,
        "remote", "def", "lib/my_app/controller.ex", 7,
    )
    .expect("Failed to insert call: Controller.index -> Accounts.list_users");
    insert_call(
        &*db,
        "MyApp.Controller", "show", 2,
        "MyApp.Accounts", "get_user", 2,
        "remote", "def", "lib/my_app/controller.ex", 15,
    )
    .expect("Failed to insert call: Controller.show -> Accounts.get_user/2");
    insert_call(
        &*db,
        "MyApp.Controller", "create", 2,
        "MyApp.Service", "process_request", 2,
        "remote", "def", "lib/my_app/controller.ex", 25,
    )
    .expect("Failed to insert call: Controller.create -> Service.process_request");

    // Accounts -> Repo
    insert_call(
        &*db,
        "MyApp.Accounts", "get_user", 1,
        "MyApp.Repo", "get", 2,
        "remote", "def", "lib/my_app/accounts.ex", 12,
    )
    .expect("Failed to insert call: Accounts.get_user/1 -> Repo.get");
    insert_call(
        &*db,
        "MyApp.Accounts", "get_user", 2,
        "MyApp.Accounts", "get_user", 1,
        "local", "def", "lib/my_app/accounts.ex", 17,
    )
    .expect("Failed to insert call: Accounts.get_user/2 -> Accounts.get_user/1");
    insert_call(
        &*db,
        "MyApp.Accounts", "list_users", 0,
        "MyApp.Repo", "all", 1,
        "remote", "def", "lib/my_app/accounts.ex", 24,
    )
    .expect("Failed to insert call: Accounts.list_users -> Repo.all");

    // Service -> Accounts
    insert_call(
        &*db,
        "MyApp.Service", "process_request", 2,
        "MyApp.Accounts", "get_user", 1,
        "remote", "def", "lib/my_app/service.ex", 12,
    )
    .expect("Failed to insert call: Service.process_request -> Accounts.get_user/1");

    // Service -> Notifier
    insert_call(
        &*db,
        "MyApp.Service", "process_request", 2,
        "MyApp.Notifier", "send_email", 2,
        "remote", "def", "lib/my_app/service.ex", 16,
    )
    .expect("Failed to insert call: Service.process_request -> Notifier.send_email");

    // Repo internal
    insert_call(
        &*db,
        "MyApp.Repo", "get", 2,
        "MyApp.Repo", "query", 2,
        "local", "def", "lib/my_app/repo.ex", 10,
    )
    .expect("Failed to insert call: Repo.get -> Repo.query");
    insert_call(
        &*db,
        "MyApp.Repo", "all", 1,
        "MyApp.Repo", "query", 2,
        "local", "def", "lib/my_app/repo.ex", 15,
    )
    .expect("Failed to insert call: Repo.all -> Repo.query");

    // Notifier internal
    insert_call(
        &*db,
        "MyApp.Notifier", "send_email", 2,
        "MyApp.Notifier", "format_message", 1,
        "local", "def", "lib/my_app/notifier.ex", 6,
    )
    .expect("Failed to insert call: Notifier.send_email -> Notifier.format_message");

    // Add alternate (shorter) path: Controller.create -> Notifier.send_email directly
    // This creates two paths to Notifier.send_email from Controller.create:
    // - Short path (1 hop): Controller.create/2 -> Notifier.send_email/2
    // - Long path (2 hops): Controller.create/2 -> Service.process_request/2 -> Notifier.send_email/2
    // Used to test that shortest path algorithm returns the shorter path
    insert_clause(&*db, "MyApp.Controller", "create", 2, 28, "lib/my_app/controller.ex", "def", 1, 1)
        .expect("Failed to insert clause for Controller.create/2 at line 28");
    insert_has_clause(&*db, "MyApp.Controller", "create", 2, 28)
        .expect("Failed to insert has_clause for Controller.create/2 at line 28");
    insert_call(
        &*db,
        "MyApp.Controller", "create", 2,
        "MyApp.Notifier", "send_email", 2,
        "remote", "def", "lib/my_app/controller.ex", 28,
    )
    .expect("Failed to insert call: Controller.create -> Notifier.send_email (direct)");

    // =======================================================================
    // CYCLE A (3 nodes): Service → Logger → Repo → Service
    // =======================================================================
    // Service.process_request -> Logger.log_query (logs the request)
    insert_call(
        &*db,
        "MyApp.Service", "process_request", 2,
        "MyApp.Logger", "log_query", 2,
        "remote", "def", "lib/my_app/service.ex", 10,
    )
    .expect("Failed to insert call: Service.process_request -> Logger.log_query");

    // Logger.log_query -> Repo.insert (persists log entry)
    insert_call(
        &*db,
        "MyApp.Logger", "log_query", 2,
        "MyApp.Repo", "insert", 1,
        "remote", "def", "lib/my_app/logger.ex", 8,
    )
    .expect("Failed to insert call: Logger.log_query -> Repo.insert");

    // Repo.insert -> Service.get_context (gets request context for audit)
    insert_call(
        &*db,
        "MyApp.Repo", "insert", 1,
        "MyApp.Service", "get_context", 1,
        "remote", "def", "lib/my_app/repo.ex", 22,
    )
    .expect("Failed to insert call: Repo.insert -> Service.get_context");

    // =======================================================================
    // CYCLE B (4 nodes): Controller → Events → Cache → Accounts → Controller
    // =======================================================================
    // Controller.create -> Events.publish (publishes create event)
    insert_call(
        &*db,
        "MyApp.Controller", "create", 2,
        "MyApp.Events", "publish", 2,
        "remote", "def", "lib/my_app/controller.ex", 27,
    )
    .expect("Failed to insert call: Controller.create -> Events.publish");

    // Events.publish -> Cache.invalidate (invalidates related cache)
    insert_call(
        &*db,
        "MyApp.Events", "publish", 2,
        "MyApp.Cache", "invalidate", 1,
        "remote", "def", "lib/my_app/events.ex", 8,
    )
    .expect("Failed to insert call: Events.publish -> Cache.invalidate");

    // Cache.invalidate -> Accounts.notify_change (notifies affected module)
    insert_call(
        &*db,
        "MyApp.Cache", "invalidate", 1,
        "MyApp.Accounts", "notify_change", 1,
        "remote", "def", "lib/my_app/cache.ex", 7,
    )
    .expect("Failed to insert call: Cache.invalidate -> Accounts.notify_change");

    // Accounts.notify_change -> Controller.handle_event (triggers UI refresh)
    insert_call(
        &*db,
        "MyApp.Accounts", "notify_change", 1,
        "MyApp.Controller", "handle_event", 1,
        "remote", "def", "lib/my_app/accounts.ex", 42,
    )
    .expect("Failed to insert call: Accounts.notify_change -> Controller.handle_event");

    // =======================================================================
    // CYCLE C (5 nodes): Notifier → Metrics → Logger → Events → Cache → Notifier
    // =======================================================================
    // Notifier.send_email -> Metrics.record (records email metric)
    insert_call(
        &*db,
        "MyApp.Notifier", "send_email", 2,
        "MyApp.Metrics", "record", 2,
        "remote", "def", "lib/my_app/notifier.ex", 9,
    )
    .expect("Failed to insert call: Notifier.send_email -> Metrics.record");

    // Metrics.record -> Logger.log_metric (logs the metric)
    insert_call(
        &*db,
        "MyApp.Metrics", "record", 2,
        "MyApp.Logger", "log_metric", 1,
        "remote", "def", "lib/my_app/metrics.ex", 8,
    )
    .expect("Failed to insert call: Metrics.record -> Logger.log_metric");

    // Logger.log_metric -> Events.emit (emits metric event)
    insert_call(
        &*db,
        "MyApp.Logger", "log_metric", 1,
        "MyApp.Events", "emit", 2,
        "remote", "def", "lib/my_app/logger.ex", 14,
    )
    .expect("Failed to insert call: Logger.log_metric -> Events.emit");

    // Events.emit -> Cache.store (caches the event)
    insert_call(
        &*db,
        "MyApp.Events", "emit", 2,
        "MyApp.Cache", "store", 2,
        "remote", "def", "lib/my_app/events.ex", 15,
    )
    .expect("Failed to insert call: Events.emit -> Cache.store");

    // Cache.store -> Notifier.on_cache_update (notifies about cache update)
    insert_call(
        &*db,
        "MyApp.Cache", "store", 2,
        "MyApp.Notifier", "on_cache_update", 1,
        "remote", "def", "lib/my_app/cache.ex", 13,
    )
    .expect("Failed to insert call: Cache.store -> Notifier.on_cache_update");

    // ========== Duplicate Detection Test Data ==========
    // Add duplicate test data as per TICKET_19 requirements

    // AST duplicates: format_name and format_display have same AST structure
    insert_clause_with_hash(
        &*db,
        "MyApp.Accounts",
        "format_name",
        1,
        50,
        "lib/my_app/accounts.ex",
        "def",
        2,
        1,
        "",
        "ast_hash_001",
        None,
    )
    .expect("Failed to insert clause for Accounts.format_name/1");
    insert_function(&*db, "MyApp.Accounts", "format_name", 1)
        .expect("Failed to insert format_name/1");
    insert_has_clause(&*db, "MyApp.Accounts", "format_name", 1, 50)
        .expect("Failed to insert has_clause for Accounts.format_name/1");

    insert_clause_with_hash(
        &*db,
        "MyApp.Controller",
        "format_display",
        1,
        60,
        "lib/my_app/controller.ex",
        "def",
        2,
        1,
        "",
        "ast_hash_001",
        None,
    )
    .expect("Failed to insert clause for Controller.format_display/1");
    insert_function(&*db, "MyApp.Controller", "format_display", 1)
        .expect("Failed to insert format_display/1");
    insert_has_clause(&*db, "MyApp.Controller", "format_display", 1, 60)
        .expect("Failed to insert has_clause for Controller.format_display/1");

    // Source duplicates: validate functions have exact same source
    insert_clause_with_hash(
        &*db,
        "MyApp.Service",
        "validate",
        1,
        70,
        "lib/my_app/service.ex",
        "def",
        1,
        1,
        "src_hash_001",
        "",
        None,
    )
    .expect("Failed to insert clause for Service.validate/1");
    insert_function(&*db, "MyApp.Service", "validate", 1)
        .expect("Failed to insert validate/1");
    insert_has_clause(&*db, "MyApp.Service", "validate", 1, 70)
        .expect("Failed to insert has_clause for Service.validate/1");

    insert_clause_with_hash(
        &*db,
        "MyApp.Repo",
        "validate",
        1,
        80,
        "lib/my_app/repo.ex",
        "def",
        1,
        1,
        "src_hash_001",
        "",
        None,
    )
    .expect("Failed to insert clause for Repo.validate/1");
    insert_function(&*db, "MyApp.Repo", "validate", 1)
        .expect("Failed to insert validate/1");
    insert_has_clause(&*db, "MyApp.Repo", "validate", 1, 80)
        .expect("Failed to insert has_clause for Repo.validate/1");

    // Generated duplicates: same AST hash but marked as generated
    insert_clause_with_hash(
        &*db,
        "MyApp.Accounts",
        "__generated__",
        0,
        90,
        "lib/my_app/accounts.ex",
        "def",
        1,
        1,
        "",
        "ast_hash_002",
        Some("phoenix"),
    )
    .expect("Failed to insert clause for Accounts.__generated__/0");
    insert_function(&*db, "MyApp.Accounts", "__generated__", 0)
        .expect("Failed to insert __generated__/0");
    insert_has_clause(&*db, "MyApp.Accounts", "__generated__", 0, 90)
        .expect("Failed to insert has_clause for Accounts.__generated__/0");

    insert_clause_with_hash(
        &*db,
        "MyApp.Controller",
        "__generated__",
        0,
        100,
        "lib/my_app/controller.ex",
        "def",
        1,
        1,
        "",
        "ast_hash_002",
        Some("phoenix"),
    )
    .expect("Failed to insert clause for Controller.__generated__/0");
    insert_function(&*db, "MyApp.Controller", "__generated__", 0)
        .expect("Failed to insert __generated__/0");
    insert_has_clause(&*db, "MyApp.Controller", "__generated__", 0, 100)
        .expect("Failed to insert has_clause for Controller.__generated__/0");

    db
}

/// Create a test database with type signature data.
///
/// Sets up an in-memory SurrealDB instance with the complete graph schema
/// and fixtures containing:
/// - One module (types_module)
/// - One function with a complex return type signature
/// - One type definition (struct)
///
/// This fixture is suitable for testing:
/// - Type signature queries
/// - Struct field traversal
/// - Function signature parsing
///
/// # Returns
/// A boxed trait object containing the configured database instance
///
/// # Panics
/// Panics if database creation or schema setup fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
pub fn surreal_type_signatures_db() -> Box<dyn Database> {
    let db = open_mem_db().expect("Failed to create in-memory database");
    schema::create_schema(&*db).expect("Failed to create schema");

    insert_module(&*db, "types_module").expect("Failed to insert types_module");

    insert_function(&*db, "types_module", "process", 1)
        .expect("Failed to insert process/1");

    // Add a spec for the function
    insert_spec(
        &*db,
        "types_module",
        "process",
        1,
        "spec",
        5,
        0,
        "@spec process(term()) :: {:ok, result} | {:error, reason}",
        &["term()"],
        &["{:ok, result}", "{:error, reason}"],
    )
    .expect("Failed to insert spec for process/1");

    insert_type(
        &*db,
        "types_module",
        "user",
        "struct",
        "{name :: string(), age :: integer()}",
    )
    .expect("Failed to insert user type");

    db
}

/// Create a test database with struct definitions.
///
/// Sets up an in-memory SurrealDB instance with the complete graph schema
/// and fixtures containing:
/// - One module (structs_module)
/// - One struct type (person)
/// - Two fields (name: string(), age: integer())
/// - Relationship edges linking the struct to its fields
///
/// This fixture is suitable for testing:
/// - Struct field queries
/// - Type definition traversal
/// - Struct composition analysis
///
/// # Returns
/// A boxed trait object containing the configured database instance
///
/// # Panics
/// Panics if database creation or schema setup fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
pub fn surreal_structs_db() -> Box<dyn Database> {
    let db = open_mem_db().expect("Failed to create in-memory database");
    schema::create_schema(&*db).expect("Failed to create schema");

    // In Elixir, struct name = module name
    insert_module(&*db, "structs_module").expect("Failed to insert structs_module");

    // The struct type definition
    insert_type(&*db, "structs_module", "structs_module", "struct", "%{name: nil, age: nil}")
        .expect("Failed to insert structs_module type");

    // Fields belong directly to the module (struct name = module name)
    insert_field(&*db, "structs_module", "name", "nil", false)
        .expect("Failed to insert name field");

    insert_field(&*db, "structs_module", "age", "nil", false)
        .expect("Failed to insert age field");

    insert_has_field(&*db, "structs_module", "name")
        .expect("Failed to create has_field relation for name");
    insert_has_field(&*db, "structs_module", "age")
        .expect("Failed to create has_field relation for age");

    db
}

/// Create a test database with type definitions for comprehensive type query testing.
///
/// Sets up an in-memory SurrealDB instance with:
/// - Two modules: module_a, module_b
/// - Three types:
///   - User struct in module_a
///   - Post struct in module_b
///   - Comment struct in module_b
///
/// This fixture is suitable for testing:
/// - Type query filtering by module pattern
/// - Type query filtering by name
/// - Type query filtering by kind
/// - Combined filtering (module + name + kind)
/// - Regex pattern matching on modules and names
/// - Sorting by module and name
/// - Limit respecting behavior
///
/// # Returns
/// A boxed trait object containing the configured database instance
///
/// # Panics
/// Panics if database creation or schema setup fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
pub fn surreal_type_db() -> Box<dyn Database> {
    let db = open_mem_db().expect("Failed to create in-memory database");
    schema::create_schema(&*db).expect("Failed to create schema");

    // Create modules
    insert_module(&*db, "module_a").expect("Failed to insert module_a");
    insert_module(&*db, "module_b").expect("Failed to insert module_b");

    // Insert types for module_a
    insert_type(&*db, "module_a", "User", "struct", "user definition")
        .expect("Failed to insert User type");

    // Insert types for module_b
    insert_type(&*db, "module_b", "Post", "struct", "post definition")
        .expect("Failed to insert Post type");
    insert_type(&*db, "module_b", "Comment", "struct", "comment definition")
        .expect("Failed to insert Comment type");

    db
}

/// Create a test database with spec data for accepts query testing.
///
/// Sets up an in-memory SurrealDB instance with:
/// - Three modules: MyApp.Accounts, MyApp.Users, MyApp.Repo
/// - Nine specs with varied input type signatures
/// - Specs with zero to multiple input types
/// - Different function arities
///
/// This fixture is suitable for testing:
/// - Pattern matching on input types (substring and regex)
/// - Array-based type matching (SurrealDB array<string> field)
/// - Module filtering
/// - Limit enforcement
/// - Empty result handling
/// - Regex validation
///
/// # Returns
/// A boxed trait object containing the configured database instance
///
/// # Panics
/// Panics if database creation or schema setup fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
pub fn surreal_accepts_db() -> Box<dyn Database> {
    let db = open_mem_db().expect("Failed to create in-memory database");
    schema::create_schema(&*db).expect("Failed to create schema");

    // Create modules
    insert_module(&*db, "MyApp.Accounts").expect("Failed to insert MyApp.Accounts");
    insert_module(&*db, "MyApp.Users").expect("Failed to insert MyApp.Users");
    insert_module(&*db, "MyApp.Repo").expect("Failed to insert MyApp.Repo");

    // Create functions
    insert_function(&*db, "MyApp.Accounts", "get_user", 1)
        .expect("Failed to insert get_user/1");
    insert_function(&*db, "MyApp.Accounts", "get_user", 2)
        .expect("Failed to insert get_user/2");
    insert_function(&*db, "MyApp.Accounts", "list_users", 0)
        .expect("Failed to insert list_users/0");
    insert_function(&*db, "MyApp.Accounts", "create_user", 1)
        .expect("Failed to insert create_user/1");
    insert_function(&*db, "MyApp.Users", "get_by_email", 1)
        .expect("Failed to insert get_by_email/1");
    insert_function(&*db, "MyApp.Users", "authenticate", 2)
        .expect("Failed to insert authenticate/2");
    insert_function(&*db, "MyApp.Repo", "get", 2)
        .expect("Failed to insert get/2");
    insert_function(&*db, "MyApp.Repo", "all", 1)
        .expect("Failed to insert all/1");
    insert_function(&*db, "MyApp.Repo", "insert", 2)
        .expect("Failed to insert insert/2");

    // Insert specs with input/return type arrays
    // 1. MyApp.Accounts.get_user/1 - single integer type
    insert_spec(
        &*db,
        "MyApp.Accounts",
        "get_user",
        1,
        "spec",
        10,
        0,
        "@spec get_user(integer()) :: {:ok, user()} | {:error, :not_found}",
        &["integer()"],
        &["{:ok, user()}", "{:error, :not_found}"],
    )
    .expect("Failed to insert get_user/1 spec");

    // 2. MyApp.Accounts.get_user/2 - multiple types including keyword()
    insert_spec(
        &*db,
        "MyApp.Accounts",
        "get_user",
        2,
        "spec",
        12,
        0,
        "@spec get_user(integer(), keyword()) :: {:ok, user()} | {:error, :not_found}",
        &["integer()", "keyword()"],
        &["{:ok, user()}", "{:error, :not_found}"],
    )
    .expect("Failed to insert get_user/2 spec");

    // 3. MyApp.Accounts.list_users/0 - zero inputs
    insert_spec(
        &*db,
        "MyApp.Accounts",
        "list_users",
        0,
        "spec",
        14,
        0,
        "@spec list_users() :: {:ok, [user()]} | {:error, reason()}",
        &[],
        &["{:ok, [user()]}", "{:error, reason()}"],
    )
    .expect("Failed to insert list_users/0 spec");

    // 4. MyApp.Accounts.create_user/1 - map type
    insert_spec(
        &*db,
        "MyApp.Accounts",
        "create_user",
        1,
        "spec",
        16,
        0,
        "@spec create_user(map()) :: {:ok, user()} | {:error, reason()}",
        &["map()"],
        &["{:ok, user()}", "{:error, reason()}"],
    )
    .expect("Failed to insert create_user/1 spec");

    // 5. MyApp.Users.get_by_email/1 - String.t() type
    insert_spec(
        &*db,
        "MyApp.Users",
        "get_by_email",
        1,
        "spec",
        20,
        0,
        "@spec get_by_email(String.t()) :: {:ok, user()} | {:error, :not_found}",
        &["String.t()"],
        &["{:ok, user()}", "{:error, :not_found}"],
    )
    .expect("Failed to insert get_by_email/1 spec");

    // 6. MyApp.Users.authenticate/2 - two String.t() types
    insert_spec(
        &*db,
        "MyApp.Users",
        "authenticate",
        2,
        "spec",
        22,
        0,
        "@spec authenticate(String.t(), String.t()) :: {:ok, token()} | {:error, reason()}",
        &["String.t()", "String.t()"],
        &["{:ok, token()}", "{:error, reason()}"],
    )
    .expect("Failed to insert authenticate/2 spec");

    // 7. MyApp.Repo.get/2 - module() and integer() types
    insert_spec(
        &*db,
        "MyApp.Repo",
        "get",
        2,
        "spec",
        30,
        0,
        "@spec get(module(), integer()) :: any() | nil",
        &["module()", "integer()"],
        &["any()", "nil"],
    )
    .expect("Failed to insert get/2 spec");

    // 8. MyApp.Repo.all/1 - Ecto.Queryable.t() type (complex type for regex testing)
    insert_spec(
        &*db,
        "MyApp.Repo",
        "all",
        1,
        "spec",
        32,
        0,
        "@spec all(Ecto.Queryable.t()) :: [any()]",
        &["Ecto.Queryable.t()"],
        &["[any()]"],
    )
    .expect("Failed to insert all/1 spec");

    // 9. MyApp.Repo.insert/2 - struct and keyword types
    insert_spec(
        &*db,
        "MyApp.Repo",
        "insert",
        2,
        "spec",
        34,
        0,
        "@spec insert(struct(), keyword()) :: {:ok, result()} | {:error, reason()}",
        &["struct()", "keyword()"],
        &["{:ok, result()}", "{:error, reason()}"],
    )
    .expect("Failed to insert insert/2 spec");

    db
}

// =============================================================================
// Tests for SurrealDB Fixture Functions
// =============================================================================

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_fixture_tests {
    use super::*;

    #[test]
    fn test_simple_create_and_select() {
        let db = open_mem_db().expect("Failed to create DB");

        // Define a simple test table
        db.execute_query_no_params(
            "DEFINE TABLE test SCHEMAFULL; DEFINE FIELD name ON test TYPE string;",
        )
        .expect("Failed to define table");

        // Create a test record
        db.execute_query_no_params("CREATE test:one SET name = 'test1';")
            .expect("Failed to create record");

        // Verify we can select it back
        let result = db
            .execute_query_no_params("SELECT * FROM test;")
            .expect("Failed to query test table");

        let rows = result.rows();
        assert_eq!(rows.len(), 1, "Should have exactly one record");

        // Verify selecting by specific ID also works
        let result2 = db
            .execute_query_no_params("SELECT * FROM test:one;")
            .expect("Failed to query specific record");
        assert_eq!(result2.rows().len(), 1, "Should find record by ID");
    }

    #[test]
    fn test_surreal_call_graph_db_creates_valid_database() {
        let db = surreal_call_graph_db();

        // Verify database is accessible by running a simple query
        let result = db.execute_query_no_params("SELECT * FROM functions LIMIT 1");
        assert!(
            result.is_ok(),
            "Should be able to query the database: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_contains_modules() {
        let db = surreal_call_graph_db();

        // Query to verify modules exist
        let result = db
            .execute_query_no_params("SELECT * FROM modules")
            .expect("Should be able to query modules");

        let rows = result.rows();
        // Should have at least 2 modules (module_a, module_b)
        assert!(
            rows.len() >= 2,
            "Should have at least 2 modules, got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_contains_functions() {
        let db = surreal_call_graph_db();

        // Query to verify functions exist
        let result = db
            .execute_query_no_params("SELECT * FROM functions")
            .expect("Should be able to query functions");

        let rows = result.rows();
        assert!(
            rows.len() >= 3,
            "Should have at least 3 functions, got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_contains_calls() {
        let db = surreal_call_graph_db();

        // Query to verify calls exist
        let result = db
            .execute_query_no_params("SELECT * FROM calls")
            .expect("Should be able to query calls");

        let rows = result.rows();
        assert!(
            rows.len() >= 2,
            "Should have at least 2 calls, got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_type_signatures_db_creates_valid_database() {
        let db = surreal_type_signatures_db();

        // Verify database is accessible
        let result = db.execute_query_no_params("SELECT * FROM types");
        assert!(result.is_ok(), "Should be able to query the database");
    }

    #[test]
    fn test_surreal_type_signatures_db_contains_types() {
        let db = surreal_type_signatures_db();

        // Query to verify types exist
        let result = db
            .execute_query_no_params("SELECT * FROM types")
            .expect("Should be able to query types");

        let rows = result.rows();
        assert!(!rows.is_empty(), "Should have type count result");
    }

    #[test]
    fn test_surreal_structs_db_creates_valid_database() {
        let db = surreal_structs_db();

        // Verify database is accessible
        let result = db.execute_query_no_params("SELECT * FROM fields");
        assert!(result.is_ok(), "Should be able to query the database");
    }

    #[test]
    fn test_surreal_structs_db_contains_fields() {
        let db = surreal_structs_db();

        // Query to verify fields exist
        let result = db
            .execute_query_no_params("SELECT * FROM fields")
            .expect("Should be able to query fields");

        let rows = result.rows();
        assert!(!rows.is_empty(), "Should have field count result");
    }

    #[test]
    fn test_surreal_structs_db_contains_has_field_relations() {
        let db = surreal_structs_db();

        // Query to verify has_field relations exist
        let result = db
            .execute_query_no_params("SELECT * FROM has_field")
            .expect("Should be able to query has_field relations");

        let rows = result.rows();
        assert!(!rows.is_empty(), "Should have has_field count result");
    }

    #[test]
    fn test_surreal_call_graph_db_complex_creates_valid_database() {
        let db = surreal_call_graph_db_complex();

        // Verify database is accessible
        let result = db.execute_query_no_params("SELECT * FROM modules LIMIT 1");
        assert!(
            result.is_ok(),
            "Should be able to query the database: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_contains_nine_modules() {
        let db = surreal_call_graph_db_complex();

        // Query to verify we have exactly 9 modules
        let result = db
            .execute_query_no_params("SELECT * FROM modules")
            .expect("Should be able to query modules");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            9,
            "Should have exactly 9 modules (Controller, Accounts, Service, Repo, Notifier, Logger, Events, Cache, Metrics), got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_contains_thirtyseven_functions() {
        let db = surreal_call_graph_db_complex();

        // Query to verify we have 37 functions:
        // - Original 16 (15 regular + 1 __struct__)
        // - 15 new for cycle testing
        // - 6 new for duplicate testing
        let result = db
            .execute_query_no_params("SELECT * FROM functions")
            .expect("Should be able to query functions");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            37,
            "Should have exactly 37 functions (16 original + 15 for cycles + 6 for duplicates), got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_contains_twentyfour_calls() {
        let db = surreal_call_graph_db_complex();

        // Query to verify we have 24 call relationships:
        // - 12 original calls
        // - 3 for Cycle A (Service → Logger → Repo → Service)
        // - 4 for Cycle B (Controller → Events → Cache → Accounts → Controller)
        // - 5 for Cycle C (Notifier → Metrics → Logger → Events → Cache → Notifier)
        let result = db
            .execute_query_no_params("SELECT * FROM calls")
            .expect("Should be able to query calls");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            24,
            "Should have exactly 24 call relationships (12 original + 12 for cycles), got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_has_multi_arity_functions() {
        let db = surreal_call_graph_db_complex();

        // Verify get_user function exists with both arity 1 and 2
        let result = db
            .execute_query_no_params("SELECT * FROM functions WHERE module_name = 'MyApp.Accounts' AND name = 'get_user'")
            .expect("Should be able to query get_user functions");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            2,
            "Should have get_user with both arity 1 and 2, got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_has_realistic_call_chains() {
        let db = surreal_call_graph_db_complex();

        // Verify Controller.show calls Accounts.get_user/2
        let result = db
            .execute_query_no_params(
                "SELECT * FROM calls WHERE in.name = 'show' AND out.name = 'get_user'",
            )
            .expect("Should be able to query specific call");

        let rows = result.rows();
        assert!(
            !rows.is_empty(),
            "Should have Controller.show -> Accounts.get_user/2 call"
        );
    }

    #[test]
    fn test_surreal_accepts_db_creates_valid_database() {
        let db = surreal_accepts_db();

        // Verify database is accessible
        let result = db.execute_query_no_params("SELECT * FROM modules LIMIT 1");
        assert!(
            result.is_ok(),
            "Should be able to query the database: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_surreal_accepts_db_contains_modules() {
        let db = surreal_accepts_db();

        // Query to verify modules exist
        let result = db
            .execute_query_no_params("SELECT * FROM modules")
            .expect("Should be able to query modules");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            3,
            "Should have exactly 3 modules (MyApp.Accounts, MyApp.Users, MyApp.Repo), got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_accepts_db_contains_specs() {
        let db = surreal_accepts_db();

        // Query to verify specs exist
        let result = db
            .execute_query_no_params("SELECT * FROM specs")
            .expect("Should be able to query specs");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            9,
            "Should have exactly 9 specs, got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_accepts_db_specs_have_input_arrays() {
        let db = surreal_accepts_db();

        // Query to verify specs have input_strings arrays
        let result = db
            .execute_query_no_params("SELECT module_name, function_name, arity, input_strings FROM specs")
            .expect("Should be able to query spec details");

        let rows = result.rows();
        // Simple check that we can query the data
        assert!(!rows.is_empty(), "Should have specs with input_strings");
    }
}
