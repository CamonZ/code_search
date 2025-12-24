//! Database schema creation and management.
//!
//! This module provides shared schema utilities used by both the import
//! and setup commands. It defines the database schema for all relations
//! and provides functions to create, check, and drop them.

use crate::db::try_create_relation;
use std::error::Error;

// Schema definitions

pub const SCHEMA_MODULES: &str = r#"
:create modules {
    project: String,
    name: String
    =>
    file: String default "",
    source: String default "unknown"
}
"#;

pub const SCHEMA_FUNCTIONS: &str = r#"
:create functions {
    project: String,
    module: String,
    name: String,
    arity: Int
    =>
    return_type: String default "",
    args: String default "",
    source: String default "unknown"
}
"#;

pub const SCHEMA_CALLS: &str = r#"
:create calls {
    project: String,
    caller_module: String,
    caller_function: String,
    callee_module: String,
    callee_function: String,
    callee_arity: Int,
    file: String,
    line: Int,
    column: Int
    =>
    call_type: String default "remote",
    caller_kind: String default "",
    callee_args: String default ""
}
"#;

pub const SCHEMA_STRUCT_FIELDS: &str = r#"
:create struct_fields {
    project: String,
    module: String,
    field: String
    =>
    default_value: String,
    required: Bool,
    inferred_type: String
}
"#;

pub const SCHEMA_FUNCTION_LOCATIONS: &str = r#"
:create function_locations {
    project: String,
    module: String,
    name: String,
    arity: Int,
    line: Int
    =>
    file: String,
    source_file_absolute: String default "",
    column: Int,
    kind: String,
    start_line: Int,
    end_line: Int,
    pattern: String default "",
    guard: String default "",
    source_sha: String default "",
    ast_sha: String default "",
    complexity: Int default 1,
    max_nesting_depth: Int default 0,
    generated_by: String default "",
    macro_source: String default ""
}
"#;

pub const SCHEMA_SPECS: &str = r#"
:create specs {
    project: String,
    module: String,
    name: String,
    arity: Int
    =>
    kind: String,
    line: Int,
    inputs_string: String default "",
    return_string: String default "",
    full: String default ""
}
"#;

pub const SCHEMA_TYPES: &str = r#"
:create types {
    project: String,
    module: String,
    name: String
    =>
    kind: String,
    params: String default "",
    line: Int,
    definition: String default ""
}
"#;

/// Result of schema creation operation
#[derive(Debug, Clone)]
pub struct SchemaCreationResult {
    pub relation: String,
    pub created: bool,
}

/// Create all database schemas.
///
/// Returns a list of all relations with their creation status.
/// If a relation already exists, returns Ok with created=false for that relation.
pub fn create_schema(
    db: &dyn crate::backend::Database,
) -> Result<Vec<SchemaCreationResult>, Box<dyn Error>> {
    let mut result = Vec::new();

    let schemas = [
        ("modules", SCHEMA_MODULES),
        ("functions", SCHEMA_FUNCTIONS),
        ("calls", SCHEMA_CALLS),
        ("struct_fields", SCHEMA_STRUCT_FIELDS),
        ("function_locations", SCHEMA_FUNCTION_LOCATIONS),
        ("specs", SCHEMA_SPECS),
        ("types", SCHEMA_TYPES),
    ];

    for (name, script) in schemas {
        let created = try_create_relation(db, script)?;
        result.push(SchemaCreationResult {
            relation: name.to_string(),
            created,
        });
    }

    Ok(result)
}

/// Get list of all relation names managed by this schema
pub fn relation_names() -> Vec<&'static str> {
    vec![
        "modules",
        "functions",
        "calls",
        "struct_fields",
        "function_locations",
        "specs",
        "types",
    ]
}

/// Get schema script for a specific relation by name
#[allow(dead_code)]
pub fn schema_for_relation(name: &str) -> Option<&'static str> {
    match name {
        "modules" => Some(SCHEMA_MODULES),
        "functions" => Some(SCHEMA_FUNCTIONS),
        "calls" => Some(SCHEMA_CALLS),
        "struct_fields" => Some(SCHEMA_STRUCT_FIELDS),
        "function_locations" => Some(SCHEMA_FUNCTION_LOCATIONS),
        "specs" => Some(SCHEMA_SPECS),
        "types" => Some(SCHEMA_TYPES),
        _ => None,
    }
}
