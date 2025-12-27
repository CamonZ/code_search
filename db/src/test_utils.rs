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
    let query = "CREATE `module`:[$name] SET name = $name, file = \"\", source = \"unknown\";";
    let params = QueryParams::new().with_str("name", name);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a function node directly into the database.
///
/// Creates a new function record with signature (module_name, name, arity).
/// The function triple (module_name, name, arity) is the natural key and
/// must be unique within the database.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module containing this function
/// * `name` - The function name
/// * `arity` - The function arity (number of parameters)
/// * `return_type` - Optional return type signature (defaults to "any()")
/// * `visibility` - Optional visibility level (defaults to "public")
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
    return_type: Option<&str>,
    visibility: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        CREATE `function`:[$module_name, $name, $arity] SET
            module_name = $module_name,
            name = $name,
            arity = $arity,
            return_type = $return_type,
            source = $source;
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("name", name)
        .with_int("arity", arity)
        .with_str("return_type", return_type.unwrap_or("any()"))
        .with_str("source", visibility.unwrap_or("public"));
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
/// * `complexity` - Code complexity metric for this clause
/// * `depth` - Nesting depth metric for this clause
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
    complexity: i64,
    depth: i64,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        CREATE clause:[$module_name, $function_name, $arity, $line] SET
            module_name = $module_name,
            function_name = $function_name,
            arity = $arity,
            line = $line,
            file = "",
            source_file_absolute = "",
            column = 0,
            kind = "",
            start_line = $line,
            end_line = $line,
            pattern = "",
            guard = "",
            source_sha = "",
            ast_sha = "",
            complexity = $complexity,
            max_nesting_depth = $depth;
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("function_name", function_name)
        .with_int("arity", arity)
        .with_int("line", line)
        .with_int("complexity", complexity)
        .with_int("depth", depth);
    db.execute_query(query, params)?;
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
        CREATE `type`:[$module_name, $name] SET
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

/// Insert a field node directly into the database.
///
/// Creates a new struct/type field record. The field natural key is
/// (module_name, type_name, name) and must be unique.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - The module containing the struct
/// * `type_name` - The struct/type name that contains this field
/// * `field_name` - The field name
/// * `field_type` - The field type specification
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the field already exists or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_field(
    db: &dyn Database,
    module_name: &str,
    type_name: &str,
    field_name: &str,
    field_type: &str,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        CREATE `field`:[$module_name, $type_name, $field_name] SET
            module_name = $module_name,
            type_name = $type_name,
            name = $field_name,
            default_value = "",
            required = false,
            inferred_type = $field_type;
    "#;
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("type_name", type_name)
        .with_str("field_name", field_name)
        .with_str("field_type", field_type);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a call relationship edge between two functions.
///
/// Creates a directed edge from caller function to callee function, recording
/// the call type (local or remote) and the line number where the call occurs.
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
/// * `line` - Line number where the call occurs (must match a clause line)
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
    line: i64,
) -> Result<(), Box<dyn Error>> {
    let query = r#"
        RELATE
            `function`:[$from_module, $from_fn, $from_arity]
            ->calls->
            `function`:[$to_module, $to_fn, $to_arity]
        SET
            call_type = $call_type,
            caller_kind = "",
            callee_args = "",
            file = "",
            line = $line,
            column = 0,
            caller_clause_id = clause:[$from_module, $from_fn, $from_arity, $line];
    "#;
    let params = QueryParams::new()
        .with_str("from_module", from_module)
        .with_str("from_fn", from_fn)
        .with_int("from_arity", from_arity)
        .with_str("to_module", to_module)
        .with_str("to_fn", to_fn)
        .with_int("to_arity", to_arity)
        .with_str("call_type", call_type)
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
/// * `entity_type` - The entity type: "function" or "type"
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
        "RELATE module:⟨$module_name⟩ ->defines-> {}:⟨$entity_id⟩;",
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
#[allow(dead_code)] // Helper for future tests
fn insert_has_clause(
    db: &dyn Database,
    function_id: &str,
    clause_id: &str,
) -> Result<(), Box<dyn Error>> {
    let query = "RELATE `function`:⟨$function_id⟩ ->has_clause-> clause:⟨$clause_id⟩;";
    let params = QueryParams::new()
        .with_str("function_id", function_id)
        .with_str("clause_id", clause_id);
    db.execute_query(query, params)?;
    Ok(())
}

/// Insert a has_field relationship edge from type to field.
///
/// Creates an edge linking a type/struct to one of its fields.
/// This relationship enables traversal of struct field definitions.
///
/// # Arguments
/// * `db` - Reference to the database instance
/// * `module_name` - Module containing the type
/// * `type_name` - Name of the type/struct
/// * `field_name` - Name of the field
///
/// # Returns
/// * `Ok(())` if insertion succeeded
/// * `Err` if the relationship cannot be created or database operation fails
#[cfg(all(any(test, feature = "test-utils"), feature = "backend-surrealdb"))]
fn insert_has_field(
    db: &dyn Database,
    module_name: &str,
    type_name: &str,
    field_name: &str,
) -> Result<(), Box<dyn Error>> {
    let query = "RELATE `type`:[$module_name, $type_name] ->has_field-> `field`:[$module_name, $type_name, $field_name];";
    let params = QueryParams::new()
        .with_str("module_name", module_name)
        .with_str("type_name", type_name)
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

    insert_function(&*db, "module_a", "foo", 1, None, Some("public"))
        .expect("Failed to insert foo/1");
    insert_function(&*db, "module_a", "bar", 2, None, Some("private"))
        .expect("Failed to insert bar/2");
    insert_function(&*db, "module_b", "baz", 0, None, Some("public"))
        .expect("Failed to insert baz/0");

    // Create clauses for each function (required for call relationships)
    // Clause lines must match the lines where calls occur
    insert_clause(&*db, "module_a", "foo", 1, 10, 1, 1)
        .expect("Failed to insert clause for foo/1 at line 10");
    insert_clause(&*db, "module_a", "bar", 2, 8, 2, 1)
        .expect("Failed to insert clause for bar/2 at line 8");
    insert_clause(&*db, "module_b", "baz", 0, 3, 1, 1)
        .expect("Failed to insert clause for baz/0 at line 3");

    // Create calls - line numbers must match the caller's clause line
    insert_call(
        &*db, "module_a", "foo", 1, "module_a", "bar", 2, "local", 10,
    )
    .expect("Failed to insert call: foo -> bar");

    // Second call from foo - need another clause at line 15
    insert_clause(&*db, "module_a", "foo", 1, 15, 1, 1)
        .expect("Failed to insert clause for foo/1 at line 15");

    insert_call(
        &*db, "module_a", "foo", 1, "module_b", "baz", 0, "remote", 15,
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

    // Controller functions (public API)
    insert_function(&*db, "MyApp.Controller", "index", 2, None, Some("public"))
        .expect("Failed to insert index/2");
    insert_function(&*db, "MyApp.Controller", "show", 2, None, Some("public"))
        .expect("Failed to insert show/2");
    insert_function(&*db, "MyApp.Controller", "create", 2, None, Some("public"))
        .expect("Failed to insert create/2");

    // Accounts functions (business logic)
    insert_function(&*db, "MyApp.Accounts", "get_user", 1, None, Some("public"))
        .expect("Failed to insert get_user/1");
    insert_function(&*db, "MyApp.Accounts", "get_user", 2, None, Some("public"))
        .expect("Failed to insert get_user/2");
    insert_function(
        &*db,
        "MyApp.Accounts",
        "list_users",
        0,
        None,
        Some("public"),
    )
    .expect("Failed to insert list_users/0");
    insert_function(
        &*db,
        "MyApp.Accounts",
        "validate_email",
        1,
        None,
        Some("private"),
    )
    .expect("Failed to insert validate_email/1");

    // Service functions
    insert_function(
        &*db,
        "MyApp.Service",
        "process_request",
        2,
        None,
        Some("public"),
    )
    .expect("Failed to insert process_request/2");
    insert_function(
        &*db,
        "MyApp.Service",
        "transform_data",
        1,
        None,
        Some("private"),
    )
    .expect("Failed to insert transform_data/1");

    // Repo functions (data access)
    insert_function(&*db, "MyApp.Repo", "get", 2, None, Some("public"))
        .expect("Failed to insert get/2");
    insert_function(&*db, "MyApp.Repo", "all", 1, None, Some("public"))
        .expect("Failed to insert all/1");
    insert_function(&*db, "MyApp.Repo", "insert", 1, None, Some("public"))
        .expect("Failed to insert insert/1");
    insert_function(&*db, "MyApp.Repo", "query", 2, None, Some("private"))
        .expect("Failed to insert query/2");

    // Notifier functions
    insert_function(
        &*db,
        "MyApp.Notifier",
        "send_email",
        2,
        None,
        Some("public"),
    )
    .expect("Failed to insert send_email/2");
    insert_function(
        &*db,
        "MyApp.Notifier",
        "format_message",
        1,
        None,
        Some("private"),
    )
    .expect("Failed to insert format_message/1");

    // Create clauses with realistic line numbers
    // Controller.index/2 - calls Accounts.list_users/0
    insert_clause(&*db, "MyApp.Controller", "index", 2, 5, 3, 2)
        .expect("Failed to insert clause for Controller.index/2");
    insert_clause(&*db, "MyApp.Controller", "index", 2, 7, 1, 1)
        .expect("Failed to insert clause for Controller.index/2 at line 7");

    // Controller.show/2 - calls Accounts.get_user/2
    insert_clause(&*db, "MyApp.Controller", "show", 2, 12, 3, 2)
        .expect("Failed to insert clause for Controller.show/2");
    insert_clause(&*db, "MyApp.Controller", "show", 2, 15, 1, 1)
        .expect("Failed to insert clause for Controller.show/2 at line 15");

    // Controller.create/2 - calls Service.process_request/2
    insert_clause(&*db, "MyApp.Controller", "create", 2, 20, 5, 3)
        .expect("Failed to insert clause for Controller.create/2");
    insert_clause(&*db, "MyApp.Controller", "create", 2, 25, 2, 2)
        .expect("Failed to insert clause for Controller.create/2 at line 25");

    // Accounts.get_user/1 - calls Repo.get/2
    insert_clause(&*db, "MyApp.Accounts", "get_user", 1, 10, 2, 1)
        .expect("Failed to insert clause for Accounts.get_user/1");
    insert_clause(&*db, "MyApp.Accounts", "get_user", 1, 12, 1, 1)
        .expect("Failed to insert clause for Accounts.get_user/1 at line 12");

    // Accounts.get_user/2 - calls get_user/1
    insert_clause(&*db, "MyApp.Accounts", "get_user", 2, 17, 2, 1)
        .expect("Failed to insert clause for Accounts.get_user/2");

    // Accounts.list_users/0 - calls Repo.all/1
    insert_clause(&*db, "MyApp.Accounts", "list_users", 0, 24, 2, 1)
        .expect("Failed to insert clause for Accounts.list_users/0");

    // Accounts.validate_email/1
    insert_clause(&*db, "MyApp.Accounts", "validate_email", 1, 30, 4, 2)
        .expect("Failed to insert clause for Accounts.validate_email/1");

    // Service.process_request/2 - calls Accounts.get_user/1 and Notifier.send_email/2
    insert_clause(&*db, "MyApp.Service", "process_request", 2, 8, 5, 3)
        .expect("Failed to insert clause for Service.process_request/2");
    insert_clause(&*db, "MyApp.Service", "process_request", 2, 12, 2, 2)
        .expect("Failed to insert clause for Service.process_request/2 at line 12");
    insert_clause(&*db, "MyApp.Service", "process_request", 2, 16, 1, 1)
        .expect("Failed to insert clause for Service.process_request/2 at line 16");

    // Service.transform_data/1
    insert_clause(&*db, "MyApp.Service", "transform_data", 1, 22, 3, 2)
        .expect("Failed to insert clause for Service.transform_data/1");

    // Repo functions
    insert_clause(&*db, "MyApp.Repo", "get", 2, 10, 2, 1)
        .expect("Failed to insert clause for Repo.get/2");
    insert_clause(&*db, "MyApp.Repo", "all", 1, 15, 2, 1)
        .expect("Failed to insert clause for Repo.all/1");
    insert_clause(&*db, "MyApp.Repo", "insert", 1, 20, 3, 2)
        .expect("Failed to insert clause for Repo.insert/1");
    insert_clause(&*db, "MyApp.Repo", "query", 2, 28, 4, 2)
        .expect("Failed to insert clause for Repo.query/2");

    // Notifier functions
    insert_clause(&*db, "MyApp.Notifier", "send_email", 2, 6, 3, 2)
        .expect("Failed to insert clause for Notifier.send_email/2");
    insert_clause(&*db, "MyApp.Notifier", "format_message", 1, 15, 2, 1)
        .expect("Failed to insert clause for Notifier.format_message/1");

    // Create call relationships (11 calls total, matching call_graph.json structure)

    // Controller -> Accounts
    insert_call(
        &*db,
        "MyApp.Controller",
        "index",
        2,
        "MyApp.Accounts",
        "list_users",
        0,
        "local",
        7,
    )
    .expect("Failed to insert call: Controller.index -> Accounts.list_users");
    insert_call(
        &*db,
        "MyApp.Controller",
        "show",
        2,
        "MyApp.Accounts",
        "get_user",
        2,
        "local",
        15,
    )
    .expect("Failed to insert call: Controller.show -> Accounts.get_user/2");
    insert_call(
        &*db,
        "MyApp.Controller",
        "create",
        2,
        "MyApp.Service",
        "process_request",
        2,
        "local",
        25,
    )
    .expect("Failed to insert call: Controller.create -> Service.process_request");

    // Accounts -> Repo
    insert_call(
        &*db,
        "MyApp.Accounts",
        "get_user",
        1,
        "MyApp.Repo",
        "get",
        2,
        "local",
        12,
    )
    .expect("Failed to insert call: Accounts.get_user/1 -> Repo.get");
    insert_call(
        &*db,
        "MyApp.Accounts",
        "get_user",
        2,
        "MyApp.Accounts",
        "get_user",
        1,
        "local",
        17,
    )
    .expect("Failed to insert call: Accounts.get_user/2 -> Accounts.get_user/1");
    insert_call(
        &*db,
        "MyApp.Accounts",
        "list_users",
        0,
        "MyApp.Repo",
        "all",
        1,
        "local",
        24,
    )
    .expect("Failed to insert call: Accounts.list_users -> Repo.all");

    // Service -> Accounts
    insert_call(
        &*db,
        "MyApp.Service",
        "process_request",
        2,
        "MyApp.Accounts",
        "get_user",
        1,
        "local",
        12,
    )
    .expect("Failed to insert call: Service.process_request -> Accounts.get_user/1");

    // Service -> Notifier
    insert_call(
        &*db,
        "MyApp.Service",
        "process_request",
        2,
        "MyApp.Notifier",
        "send_email",
        2,
        "remote",
        16,
    )
    .expect("Failed to insert call: Service.process_request -> Notifier.send_email");

    // Repo internal
    insert_call(
        &*db,
        "MyApp.Repo",
        "get",
        2,
        "MyApp.Repo",
        "query",
        2,
        "local",
        10,
    )
    .expect("Failed to insert call: Repo.get -> Repo.query");
    insert_call(
        &*db,
        "MyApp.Repo",
        "all",
        1,
        "MyApp.Repo",
        "query",
        2,
        "local",
        15,
    )
    .expect("Failed to insert call: Repo.all -> Repo.query");

    // Notifier internal
    insert_call(
        &*db,
        "MyApp.Notifier",
        "send_email",
        2,
        "MyApp.Notifier",
        "format_message",
        1,
        "local",
        6,
    )
    .expect("Failed to insert call: Notifier.send_email -> Notifier.format_message");

    // Add alternate (shorter) path: Controller.create -> Notifier.send_email directly
    // This creates two paths to Notifier.send_email from Controller.create:
    // - Short path (1 hop): Controller.create/2 -> Notifier.send_email/2
    // - Long path (2 hops): Controller.create/2 -> Service.process_request/2 -> Notifier.send_email/2
    // Used to test that shortest path algorithm returns the shorter path
    insert_clause(&*db, "MyApp.Controller", "create", 2, 28, 1, 1)
        .expect("Failed to insert clause for Controller.create/2 at line 28");
    insert_call(
        &*db,
        "MyApp.Controller",
        "create",
        2,
        "MyApp.Notifier",
        "send_email",
        2,
        "remote",
        28,
    )
    .expect("Failed to insert call: Controller.create -> Notifier.send_email (direct)");

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

    insert_function(
        &*db,
        "types_module",
        "process",
        1,
        Some("{ok, result} | {error, reason}"),
        Some("public"),
    )
    .expect("Failed to insert process/1");

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

    insert_module(&*db, "structs_module").expect("Failed to insert structs_module");

    insert_type(&*db, "structs_module", "person", "struct", "{name, age}")
        .expect("Failed to insert person type");

    insert_field(&*db, "structs_module", "person", "name", "string()")
        .expect("Failed to insert name field");

    insert_field(&*db, "structs_module", "person", "age", "integer()")
        .expect("Failed to insert age field");

    insert_has_field(&*db, "structs_module", "person", "name")
        .expect("Failed to create has_field relation for name");
    insert_has_field(&*db, "structs_module", "person", "age")
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
        let result = db.execute_query_no_params("SELECT * FROM `function` LIMIT 1");
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
            .execute_query_no_params("SELECT * FROM `module`")
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
            .execute_query_no_params("SELECT * FROM `function`")
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
        let result = db.execute_query_no_params("SELECT * FROM `type`");
        assert!(result.is_ok(), "Should be able to query the database");
    }

    #[test]
    fn test_surreal_type_signatures_db_contains_types() {
        let db = surreal_type_signatures_db();

        // Query to verify types exist
        let result = db
            .execute_query_no_params("SELECT * FROM `type`")
            .expect("Should be able to query types");

        let rows = result.rows();
        assert!(!rows.is_empty(), "Should have type count result");
    }

    #[test]
    fn test_surreal_structs_db_creates_valid_database() {
        let db = surreal_structs_db();

        // Verify database is accessible
        let result = db.execute_query_no_params("SELECT * FROM `field`");
        assert!(result.is_ok(), "Should be able to query the database");
    }

    #[test]
    fn test_surreal_structs_db_contains_fields() {
        let db = surreal_structs_db();

        // Query to verify fields exist
        let result = db
            .execute_query_no_params("SELECT * FROM `field`")
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
        let result = db.execute_query_no_params("SELECT * FROM `module` LIMIT 1");
        assert!(
            result.is_ok(),
            "Should be able to query the database: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_contains_five_modules() {
        let db = surreal_call_graph_db_complex();

        // Query to verify we have exactly 5 modules
        let result = db
            .execute_query_no_params("SELECT * FROM `module`")
            .expect("Should be able to query modules");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            5,
            "Should have exactly 5 modules (Controller, Accounts, Service, Repo, Notifier), got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_contains_fifteen_functions() {
        let db = surreal_call_graph_db_complex();

        // Query to verify we have 15 functions
        let result = db
            .execute_query_no_params("SELECT * FROM `function`")
            .expect("Should be able to query functions");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            15,
            "Should have exactly 15 functions, got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_contains_twelve_calls() {
        let db = surreal_call_graph_db_complex();

        // Query to verify we have 12 call relationships (11 original + 1 direct path for shortest path testing)
        let result = db
            .execute_query_no_params("SELECT * FROM calls")
            .expect("Should be able to query calls");

        let rows = result.rows();
        assert_eq!(
            rows.len(),
            12,
            "Should have exactly 12 call relationships, got {}",
            rows.len()
        );
    }

    #[test]
    fn test_surreal_call_graph_db_complex_has_multi_arity_functions() {
        let db = surreal_call_graph_db_complex();

        // Verify get_user function exists with both arity 1 and 2
        let result = db
            .execute_query_no_params("SELECT * FROM `function` WHERE module_name = 'MyApp.Accounts' AND name = 'get_user'")
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
}
