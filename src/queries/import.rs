use std::error::Error;

use cozo::{DataValue, DbInstance};
use serde::Serialize;
use thiserror::Error;

use crate::queries::import_models::CallGraph;
use crate::db::{escape_string, run_query, run_query_no_params, try_create_relation, Params};

/// Chunk size for batch database imports
const IMPORT_CHUNK_SIZE: usize = 500;

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("Failed to read call graph file '{path}': {message}")]
    FileReadFailed { path: String, message: String },

    #[error("Failed to parse call graph JSON: {message}")]
    JsonParseFailed { message: String },

    #[error("Schema creation failed for '{relation}': {message}")]
    SchemaCreationFailed { relation: String, message: String },

    #[error("Failed to clear data: {message}")]
    ClearFailed { message: String },

    #[error("Failed to import {data_type}: {message}")]
    ImportFailed { data_type: String, message: String },
}

/// Result of the import command execution
#[derive(Debug, Default, Serialize)]
pub struct ImportResult {
    pub schemas: SchemaResult,
    pub cleared: bool,
    pub modules_imported: usize,
    pub functions_imported: usize,
    pub calls_imported: usize,
    pub structs_imported: usize,
    pub function_locations_imported: usize,
}

/// Result of schema creation
#[derive(Debug, Default, Serialize)]
pub struct SchemaResult {
    pub created: Vec<String>,
    pub already_existed: Vec<String>,
}

// Schema definitions

const SCHEMA_MODULES: &str = r#"
:create modules {
    project: String,
    name: String
    =>
    file: String default "",
    source: String default "unknown"
}
"#;

const SCHEMA_FUNCTIONS: &str = r#"
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

const SCHEMA_CALLS: &str = r#"
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
    call_type: String default "remote"
}
"#;

const SCHEMA_STRUCT_FIELDS: &str = r#"
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

const SCHEMA_FUNCTION_LOCATIONS: &str = r#"
:create function_locations {
    project: String,
    module: String,
    name: String,
    arity: Int
    =>
    file: String,
    column: Int,
    kind: String,
    start_line: Int,
    end_line: Int
}
"#;

pub fn create_schema(db: &DbInstance) -> Result<SchemaResult, Box<dyn Error>> {
    let mut result = SchemaResult::default();

    let schemas = [
        ("modules", SCHEMA_MODULES),
        ("functions", SCHEMA_FUNCTIONS),
        ("calls", SCHEMA_CALLS),
        ("struct_fields", SCHEMA_STRUCT_FIELDS),
        ("function_locations", SCHEMA_FUNCTION_LOCATIONS),
    ];

    for (name, script) in schemas {
        match try_create_relation(db, script) {
            Ok(true) => result.created.push(name.to_string()),
            Ok(false) => result.already_existed.push(name.to_string()),
            Err(e) => {
                return Err(ImportError::SchemaCreationFailed {
                    relation: name.to_string(),
                    message: e.to_string(),
                }
                .into())
            }
        }
    }

    Ok(result)
}

pub fn clear_project_data(db: &DbInstance, project: &str) -> Result<(), Box<dyn Error>> {
    // Delete all data for this project from each table
    // Using :rm with a query that selects rows matching the project
    let tables = [
        ("modules", "project, name"),
        ("functions", "project, module, name, arity"),
        ("calls", "project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column"),
        ("struct_fields", "project, module, field"),
        ("function_locations", "project, module, name, arity"),
    ];

    for (table, keys) in tables {
        let script = format!(
            r#"
            ?[{keys}] := *{table}{{project: $project, {keys}}}
            :rm {table} {{{keys}}}
            "#,
            table = table,
            keys = keys
        );

        let mut params = Params::new();
        params.insert("project".to_string(), DataValue::Str(project.into()));

        run_query(db, &script, params).map_err(|e| ImportError::ClearFailed {
            message: format!("Failed to clear {}: {}", table, e),
        })?;
    }

    Ok(())
}

/// Import rows in chunks into a CozoDB table
fn import_rows(
    db: &DbInstance,
    rows: Vec<String>,
    columns: &str,
    table_spec: &str,
    data_type: &str,
) -> Result<usize, Box<dyn Error>> {
    if rows.is_empty() {
        return Ok(0);
    }

    for chunk in rows.chunks(IMPORT_CHUNK_SIZE) {
        let script = format!(
            r#"
            ?[{columns}] <- [{rows}]
            :put {table_spec}
            "#,
            columns = columns,
            rows = chunk.join(", "),
            table_spec = table_spec
        );

        run_query_no_params(db, &script).map_err(|e| ImportError::ImportFailed {
            data_type: data_type.to_string(),
            message: e.to_string(),
        })?;
    }

    Ok(rows.len())
}

pub fn import_modules(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let rows: Vec<String> = graph
        .type_signatures
        .keys()
        .map(|m| {
            format!(
                r#"["{}", "{}", "", "unknown"]"#,
                escape_string(project),
                escape_string(m),
            )
        })
        .collect();

    import_rows(
        db,
        rows,
        "project, name, file, source",
        "modules { project, name => file, source }",
        "modules",
    )
}

pub fn import_functions(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let escaped_project = escape_string(project);
    let mut rows = Vec::new();

    for (module, functions) in &graph.type_signatures {
        for (_func_key, sig) in functions {
            let return_type = sig
                .clauses
                .first()
                .map(|c| c.return_type.clone())
                .unwrap_or_default();

            let args = sig
                .clauses
                .first()
                .map(|c| c.args.join(", "))
                .unwrap_or_default();

            rows.push(format!(
                r#"["{}", "{}", "{}", {}, "{}", "{}", "unknown"]"#,
                escaped_project,
                escape_string(module),
                escape_string(&sig.name),
                sig.arity,
                escape_string(&return_type),
                escape_string(&args),
            ));
        }
    }

    import_rows(
        db,
        rows,
        "project, module, name, arity, return_type, args, source",
        "functions { project, module, name, arity => return_type, args, source }",
        "functions",
    )
}

pub fn import_calls(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let escaped_project = escape_string(project);
    let rows: Vec<String> = graph
        .calls
        .iter()
        .map(|call| {
            format!(
                r#"["{}", "{}", "{}", "{}", "{}", {}, "{}", {}, {}, "{}"]"#,
                escaped_project,
                escape_string(&call.caller.module),
                escape_string(call.caller.function.as_deref().unwrap_or("<module>")),
                escape_string(&call.callee.module),
                escape_string(&call.callee.function),
                call.callee.arity,
                escape_string(&call.caller.file),
                call.caller.line.unwrap_or(0),
                call.caller.column.unwrap_or(0),
                escape_string(&call.call_type)
            )
        })
        .collect();

    import_rows(
        db,
        rows,
        "project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column, call_type",
        "calls { project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column => call_type }",
        "calls",
    )
}

pub fn import_structs(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let escaped_project = escape_string(project);
    let mut rows = Vec::new();

    for (module, def) in &graph.structs {
        for field in &def.fields {
            let inferred_type = field.inferred_type.as_deref().unwrap_or("");
            rows.push(format!(
                r#"["{}", "{}", "{}", "{}", {}, "{}"]"#,
                escaped_project,
                escape_string(module),
                escape_string(&field.field),
                escape_string(&field.default),
                field.required,
                escape_string(inferred_type)
            ));
        }
    }

    import_rows(
        db,
        rows,
        "project, module, field, default_value, required, inferred_type",
        "struct_fields { project, module, field => default_value, required, inferred_type }",
        "struct_fields",
    )
}

pub fn import_function_locations(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let escaped_project = escape_string(project);
    let mut rows = Vec::new();

    for (module, functions) in &graph.function_locations {
        for (_func_key, loc) in functions {
            rows.push(format!(
                r#"["{}", "{}", "{}", {}, "{}", {}, "{}", {}, {}]"#,
                escaped_project,
                escape_string(module),
                escape_string(&loc.name),
                loc.arity,
                escape_string(&loc.file),
                loc.column.unwrap_or(0),
                escape_string(&loc.kind),
                loc.start_line,
                loc.end_line
            ));
        }
    }

    import_rows(
        db,
        rows,
        "project, module, name, arity, file, column, kind, start_line, end_line",
        "function_locations { project, module, name, arity => file, column, kind, start_line, end_line }",
        "function_locations",
    )
}
