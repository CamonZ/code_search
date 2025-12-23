use std::error::Error;

use cozo::{DataValue, DbInstance};
use serde::Serialize;
use thiserror::Error;

use crate::db::{escape_string, escape_string_single, run_query, run_query_no_params, Params};
use crate::queries::import_models::CallGraph;
use crate::queries::schema;

/// Chunk size for batch database imports
const IMPORT_CHUNK_SIZE: usize = 500;

#[derive(Error, Debug)]
pub enum ImportError {
    #[error("Failed to read call graph file '{path}': {message}")]
    FileReadFailed { path: String, message: String },

    #[error("Failed to parse call graph JSON: {message}")]
    JsonParseFailed { message: String },

    #[allow(dead_code)]
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
    pub specs_imported: usize,
    pub types_imported: usize,
}

/// Result of schema creation
#[derive(Debug, Default, Serialize)]
pub struct SchemaResult {
    pub created: Vec<String>,
    pub already_existed: Vec<String>,
}

pub fn create_schema(db: &DbInstance) -> Result<SchemaResult, Box<dyn Error>> {
    let mut result = SchemaResult::default();

    let schema_results = schema::create_schema(db)?;

    for schema_result in schema_results {
        if schema_result.created {
            result.created.push(schema_result.relation);
        } else {
            result.already_existed.push(schema_result.relation);
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
        ("function_locations", "project, module, name, arity, line"),
        ("specs", "project, module, name, arity"),
        ("types", "project, module, name"),
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
    // Collect unique modules from all data sources
    let mut modules = std::collections::HashSet::new();
    modules.extend(graph.specs.keys().cloned());
    modules.extend(graph.function_locations.keys().cloned());
    modules.extend(graph.structs.keys().cloned());
    modules.extend(graph.types.keys().cloned());

    let rows: Vec<String> = modules
        .iter()
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

    // Import functions from specs data
    for (module, specs) in &graph.specs {
        for spec in specs {
            // Use first clause only
            let (return_type, args) = spec
                .clauses
                .first()
                .map(|c| (c.return_strings.join(" | "), c.input_strings.join(", ")))
                .unwrap_or_default();

            rows.push(format!(
                r#"["{}", "{}", "{}", {}, "{}", "{}", "unknown"]"#,
                escaped_project,
                escape_string(module),
                escape_string(&spec.name),
                spec.arity,
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
            let caller_kind = call.caller.kind.as_deref().unwrap_or("");
            let callee_args = call.callee.args.as_deref().unwrap_or("");

            format!(
                r#"["{}", "{}", "{}", "{}", "{}", {}, "{}", {}, {}, "{}", "{}", '{}']"#,
                escaped_project,
                escape_string(&call.caller.module),
                escape_string(call.caller.function.as_deref().unwrap_or("<module>")),
                escape_string(&call.callee.module),
                escape_string(&call.callee.function),
                call.callee.arity,
                escape_string(&call.caller.file),
                call.caller.line.unwrap_or(0),
                call.caller.column.unwrap_or(0),
                escape_string(&call.call_type),
                escape_string(caller_kind),
                escape_string_single(callee_args),
            )
        })
        .collect();

    import_rows(
        db,
        rows,
        "project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column, call_type, caller_kind, callee_args",
        "calls { project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column => call_type, caller_kind, callee_args }",
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
                r#"["{}", "{}", '{}', '{}', {}, "{}"]"#,
                escaped_project,
                escape_string(module),
                escape_string_single(&field.field),
                escape_string_single(&field.default),
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
        for loc in functions.values() {
            // Use deserialized fields directly from the JSON
            let name = &loc.name;
            let arity = loc.arity;
            let line = loc.line;

            let source_file_absolute = loc.source_file_absolute.as_deref().unwrap_or("");
            let pattern = loc.pattern.as_deref().unwrap_or("");
            let guard = loc.guard.as_deref().unwrap_or("");
            let source_sha = loc.source_sha.as_deref().unwrap_or("");
            let ast_sha = loc.ast_sha.as_deref().unwrap_or("");
            let generated_by = loc.generated_by.as_deref().unwrap_or("");
            let macro_source = loc.macro_source.as_deref().unwrap_or("");

            rows.push(format!(
                r#"["{}", "{}", "{}", {}, {}, "{}", "{}", {}, "{}", {}, {}, '{}', '{}', "{}", "{}", {}, {}, "{}", "{}"]"#,
                escaped_project,
                escape_string(module),
                escape_string(name),
                arity,
                line,
                escape_string(loc.file.as_deref().unwrap_or("")),
                escape_string(source_file_absolute),
                loc.column.unwrap_or(0),
                escape_string(&loc.kind),
                loc.start_line,
                loc.end_line,
                escape_string_single(pattern),
                escape_string_single(guard),
                escape_string(source_sha),
                escape_string(ast_sha),
                loc.complexity,
                loc.max_nesting_depth,
                escape_string(generated_by),
                escape_string(macro_source),
            ));
        }
    }

    import_rows(
        db,
        rows,
        "project, module, name, arity, line, file, source_file_absolute, column, kind, start_line, end_line, pattern, guard, source_sha, ast_sha, complexity, max_nesting_depth, generated_by, macro_source",
        "function_locations { project, module, name, arity, line => file, source_file_absolute, column, kind, start_line, end_line, pattern, guard, source_sha, ast_sha, complexity, max_nesting_depth, generated_by, macro_source }",
        "function_locations",
    )
}

pub fn import_specs(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let escaped_project = escape_string(project);
    let mut rows = Vec::new();

    for (module, specs) in &graph.specs {
        for spec in specs {
            // Use first clause only (as per ticket recommendation)
            let (inputs_string, return_string, full) = spec
                .clauses
                .first()
                .map(|c| {
                    (
                        c.input_strings.join(", "),
                        c.return_strings.join(" | "),
                        c.full.clone(),
                    )
                })
                .unwrap_or_default();

            rows.push(format!(
                r#"["{}", "{}", "{}", {}, "{}", {}, "{}", "{}", "{}"]"#,
                escaped_project,
                escape_string(module),
                escape_string(&spec.name),
                spec.arity,
                escape_string(&spec.kind),
                spec.line,
                escape_string(&inputs_string),
                escape_string(&return_string),
                escape_string(&full),
            ));
        }
    }

    import_rows(
        db,
        rows,
        "project, module, name, arity, kind, line, inputs_string, return_string, full",
        "specs { project, module, name, arity => kind, line, inputs_string, return_string, full }",
        "specs",
    )
}

pub fn import_types(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let escaped_project = escape_string(project);
    let mut rows = Vec::new();

    for (module, types) in &graph.types {
        for type_def in types {
            let params = type_def.params.join(", ");

            rows.push(format!(
                r#"["{}", "{}", "{}", "{}", "{}", {}, '{}']"#,
                escaped_project,
                escape_string(module),
                escape_string(&type_def.name),
                escape_string(&type_def.kind),
                escape_string(&params),
                type_def.line,
                escape_string_single(&type_def.definition),
            ));
        }
    }

    import_rows(
        db,
        rows,
        "project, module, name, kind, params, line, definition",
        "types { project, module, name => kind, params, line, definition }",
        "types",
    )
}

/// Import a parsed CallGraph into the database.
///
/// Creates schemas and imports all data (modules, functions, calls, structs, locations).
/// This is the core import logic used by both the CLI command and test utilities.
pub fn import_graph(
    db: &DbInstance,
    project: &str,
    graph: &CallGraph,
) -> Result<ImportResult, Box<dyn Error>> {
    let mut result = ImportResult::default();

    result.schemas = create_schema(db)?;
    result.modules_imported = import_modules(db, project, graph)?;
    result.functions_imported = import_functions(db, project, graph)?;
    result.calls_imported = import_calls(db, project, graph)?;
    result.structs_imported = import_structs(db, project, graph)?;
    result.function_locations_imported = import_function_locations(db, project, graph)?;
    result.specs_imported = import_specs(db, project, graph)?;
    result.types_imported = import_types(db, project, graph)?;

    Ok(result)
}

/// Import a JSON string directly into the database.
///
/// Convenience wrapper for tests that parses JSON and calls `import_graph`.
#[cfg(any(test, feature = "test-utils"))]
pub fn import_json_str(
    db: &DbInstance,
    content: &str,
    project: &str,
) -> Result<ImportResult, Box<dyn Error>> {
    let graph: CallGraph =
        serde_json::from_str(content).map_err(|e| ImportError::JsonParseFailed {
            message: e.to_string(),
        })?;

    import_graph(db, project, &graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{extract_string, open_db};
    use tempfile::NamedTempFile;

    // Test deserialization with all new fields present
    #[test]
    fn test_function_location_deserialize_with_new_fields() {
        let json = r#"{
            "name": "test_func",
            "arity": 2,
            "kind": "def",
            "line": 10,
            "start_line": 10,
            "end_line": 15,
            "complexity": 5,
            "max_nesting_depth": 3,
            "generated_by": "Ecto.Schema",
            "macro_source": "ecto/schema.ex"
        }"#;

        let result: crate::queries::import_models::FunctionLocation =
            serde_json::from_str(json).expect("Deserialization should succeed");

        assert_eq!(result.complexity, 5);
        assert_eq!(result.max_nesting_depth, 3);
        assert_eq!(result.generated_by, Some("Ecto.Schema".to_string()));
        assert_eq!(result.macro_source, Some("ecto/schema.ex".to_string()));
    }

    // Test deserialization without optional fields (backward compatibility)
    #[test]
    fn test_function_location_deserialize_without_new_fields() {
        let json = r#"{
            "name": "test_func",
            "arity": 2,
            "kind": "def",
            "line": 10,
            "start_line": 10,
            "end_line": 15
        }"#;

        let result: crate::queries::import_models::FunctionLocation =
            serde_json::from_str(json).expect("Deserialization should succeed");

        // Should use defaults
        assert_eq!(result.complexity, 1); // default_complexity
        assert_eq!(result.max_nesting_depth, 0); // default
        assert_eq!(result.generated_by, None); // default
        assert_eq!(result.macro_source, None); // default
    }

    // Test deserialization with empty string values
    #[test]
    fn test_function_location_deserialize_empty_strings() {
        let json = r#"{
            "name": "test_func",
            "arity": 2,
            "kind": "def",
            "line": 10,
            "start_line": 10,
            "end_line": 15,
            "complexity": 1,
            "max_nesting_depth": 0,
            "generated_by": "",
            "macro_source": ""
        }"#;

        let result: crate::queries::import_models::FunctionLocation =
            serde_json::from_str(json).expect("Deserialization should succeed");

        // Empty strings should deserialize to None or empty string
        assert_eq!(result.complexity, 1);
        assert_eq!(result.max_nesting_depth, 0);
        // Empty strings should parse as Some("") not None
        assert_eq!(result.generated_by, Some("".to_string()));
        assert_eq!(result.macro_source, Some("".to_string()));
    }

    // Test import and database storage of new fields
    #[test]
    fn test_import_function_locations_with_new_fields() {
        let json = r#"{
            "structs": {},
            "function_locations": {
                "MyApp.Accounts": {
                    "process_data/2:20": {
                        "name": "process_data",
                        "arity": 2,
                        "file": "lib/accounts.ex",
                        "column": 5,
                        "kind": "def",
                        "line": 20,
                        "start_line": 20,
                        "end_line": 35,
                        "pattern": null,
                        "guard": null,
                        "source_sha": "",
                        "ast_sha": "",
                        "complexity": 7,
                        "max_nesting_depth": 4,
                        "generated_by": "Phoenix.Endpoint",
                        "macro_source": "phoenix/endpoint.ex"
                    }
                }
            },
            "calls": [],
            "specs": {},
            "types": {}
        }"#;

        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        let result = import_json_str(&db, json, "test_project").expect("Import should succeed");

        // Verify import succeeded
        assert_eq!(result.function_locations_imported, 1);

        // Verify modules were created (MyApp.Accounts is inferred from function_locations)
        assert!(result.modules_imported > 0);

        // If we got here, the new fields were successfully serialized and stored in the database
        // The fact that import_graph succeeded means:
        // 1. JSON deserialization worked with the new fields
        // 2. import_function_locations() successfully formatted and inserted rows with 4 new fields
        // 3. CozoDB schema accepted the data
    }

    // Test import of struct fields with string-quoted atom syntax
    #[test]
    fn test_import_struct_fields_with_string_quoted_atoms() {
        let json = r#"{
            "structs": {
                "MyApp.User": {
                    "fields": [
                        {
                            "field": "name",
                            "default": "nil",
                            "required": false,
                            "inferred_type": "String.t()"
                        },
                        {
                            "field": ":\"user.id\"",
                            "default": "nil",
                            "required": false,
                            "inferred_type": "integer()"
                        },
                        {
                            "field": ":\"first-name\"",
                            "default": ":\"foo.bar\"",
                            "required": true,
                            "inferred_type": "String.t()"
                        }
                    ]
                }
            },
            "function_locations": {},
            "calls": [],
            "specs": {},
            "types": {}
        }"#;

        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        let result = import_json_str(&db, json, "test_project").expect("Import should succeed");

        // Verify import succeeded
        assert_eq!(result.structs_imported, 3);

        // Query the database to see what was actually stored
        let query = r#"
            ?[field, default_value] := *struct_fields{
                project: "test_project",
                module: "MyApp.User",
                field,
                default_value
            }
        "#;
        let rows = run_query_no_params(&db, query).expect("Query should succeed");

        // Extract field names and defaults
        let mut fields: Vec<(String, String)> = rows.rows.iter()
            .filter_map(|row| {
                let field = extract_string(&row[0])?;
                let default = extract_string(&row[1])?;
                Some((field, default))
            })
            .collect();
        fields.sort();

        // Verify the string-quoted atom syntax is preserved in both field names and defaults
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].0, r#":"first-name""#);
        assert_eq!(fields[0].1, r#":"foo.bar""#);
        assert_eq!(fields[1].0, r#":"user.id""#);
        assert_eq!(fields[1].1, "nil");
        assert_eq!(fields[2].0, "name");
        assert_eq!(fields[2].1, "nil");
    }

    // Test import of types with string-quoted atoms in definition
    #[test]
    fn test_import_types_with_string_quoted_atoms() {
        let json = r#"{
            "structs": {},
            "function_locations": {},
            "calls": [],
            "specs": {},
            "types": {
                "MyModule": [
                    {
                        "name": "status",
                        "kind": "type",
                        "params": [],
                        "line": 5,
                        "definition": "@type status() :: :pending | :active | :\"special.status\""
                    },
                    {
                        "name": "config",
                        "kind": "type",
                        "params": [],
                        "line": 10,
                        "definition": "@type config() :: %{:\"api.key\" => String.t()}"
                    }
                ]
            }
        }"#;

        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        let result = import_json_str(&db, json, "test_project").expect("Import should succeed");

        // Verify import succeeded
        assert_eq!(result.types_imported, 2);

        // Query the database to see what was actually stored
        let query = r#"
            ?[name, definition] := *types{
                project: "test_project",
                module: "MyModule",
                name,
                definition
            }
        "#;
        let rows = run_query_no_params(&db, query).expect("Query should succeed");

        // Extract type definitions
        let mut types: Vec<(String, String)> = rows.rows.iter()
            .filter_map(|row| {
                let name = extract_string(&row[0])?;
                let definition = extract_string(&row[1])?;
                Some((name, definition))
            })
            .collect();
        types.sort();

        // Verify the string-quoted atom syntax is preserved in definitions
        assert_eq!(types.len(), 2);
        assert_eq!(types[0].0, "config");
        assert_eq!(types[0].1, r#"@type config() :: %{:"api.key" => String.t()}"#);
        assert_eq!(types[1].0, "status");
        assert_eq!(types[1].1, r#"@type status() :: :pending | :active | :"special.status""#);
    }
}
