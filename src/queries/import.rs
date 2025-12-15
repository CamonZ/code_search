use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::schema::{
    ALL_RELATIONS, CALLS, FUNCTIONS, FUNCTION_LOCATIONS, MODULES, SPECS, STRUCT_FIELDS, TYPES,
};
use crate::queries::import_models::CallGraph;

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
    pub cleared: bool,
    pub modules_imported: usize,
    pub functions_imported: usize,
    pub calls_imported: usize,
    pub structs_imported: usize,
    pub function_locations_imported: usize,
    pub specs_imported: usize,
    pub types_imported: usize,
}

pub fn clear_project_data(db: &dyn DatabaseBackend, project: &str) -> Result<(), Box<dyn Error>> {
    // Delete all data for this project from each table using the generic interface
    for relation in ALL_RELATIONS {
        db.delete_by_project(relation, project)
            .map_err(|e| ImportError::ClearFailed {
                message: format!("Failed to clear {}: {}", relation.name, e),
            })?;
    }

    Ok(())
}

pub fn import_modules(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    // Collect unique modules from all data sources
    let mut modules = std::collections::HashSet::new();
    modules.extend(graph.specs.keys().cloned());
    modules.extend(graph.function_locations.keys().cloned());
    modules.extend(graph.structs.keys().cloned());
    modules.extend(graph.types.keys().cloned());

    let rows: Vec<Vec<DataValue>> = modules
        .iter()
        .map(|m| {
            vec![
                DataValue::Str(project.into()),   // project
                DataValue::Str(m.clone().into()), // name
                DataValue::Str("".into()),        // file
                DataValue::Str("unknown".into()), // source
            ]
        })
        .collect();

    db.insert_rows(&MODULES, rows)
}

pub fn import_functions(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut rows = Vec::new();

    // Import functions from specs data
    for (module, specs) in &graph.specs {
        for spec in specs {
            // Use first clause only
            let (return_type, args) = spec
                .clauses
                .first()
                .map(|c| (c.return_string.clone(), c.inputs_string.join(", ")))
                .unwrap_or_default();

            rows.push(vec![
                DataValue::Str(project.into()),           // project
                DataValue::Str(module.clone().into()),    // module
                DataValue::Str(spec.name.clone().into()), // name
                DataValue::from(spec.arity as i64),       // arity
                DataValue::Str(return_type.into()),       // return_type
                DataValue::Str(args.into()),              // args
                DataValue::Str("unknown".into()),         // source
            ]);
        }
    }

    db.insert_rows(&FUNCTIONS, rows)
}

pub fn import_calls(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let rows: Vec<Vec<DataValue>> = graph
        .calls
        .iter()
        .map(|call| {
            let caller_kind = call.caller.kind.as_deref().unwrap_or("");
            let callee_args = call.callee.args.as_deref().unwrap_or("");

            vec![
                DataValue::Str(project.into()),                    // project
                DataValue::Str(call.caller.module.clone().into()), // caller_module
                DataValue::Str(call.caller.function.clone().unwrap_or_default().into()), // caller_function
                DataValue::Str(call.callee.module.clone().into()), // callee_module
                DataValue::Str(call.callee.function.clone().into()), // callee_function
                DataValue::from(call.callee.arity as i64),         // callee_arity
                DataValue::Str(call.caller.file.clone().into()),   // file
                DataValue::from(call.caller.line.unwrap_or(0) as i64), // line
                DataValue::from(call.caller.column.unwrap_or(0) as i64), // column
                DataValue::Str(call.call_type.clone().into()),     // call_type
                DataValue::Str(caller_kind.into()),                // caller_kind
                DataValue::Str(callee_args.into()),                // callee_args
            ]
        })
        .collect();

    db.insert_rows(&CALLS, rows)
}

pub fn import_structs(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut rows = Vec::new();

    for (module, def) in &graph.structs {
        for field in &def.fields {
            let inferred_type = field.inferred_type.as_deref().unwrap_or("");
            rows.push(vec![
                DataValue::Str(project.into()),               // project
                DataValue::Str(module.clone().into()),        // module
                DataValue::Str(field.field.clone().into()),   // field
                DataValue::Str(field.default.clone().into()), // default_value
                DataValue::Bool(field.required),              // required
                DataValue::Str(inferred_type.into()),         // inferred_type
            ]);
        }
    }

    db.insert_rows(&STRUCT_FIELDS, rows)
}

/// Parse function key in format "name/arity:line" into (name, arity, line).
///
/// Returns None if the key doesn't match the expected format.
fn parse_function_key(key: &str) -> Option<(String, u32, u32)> {
    // Format: "function_name/arity:line"
    // Example: "keep_values/2:224"
    let colon_pos = key.rfind(':')?;
    let line: u32 = key[colon_pos + 1..].parse().ok()?;

    let before_colon = &key[..colon_pos];
    let slash_pos = before_colon.rfind('/')?;
    let arity: u32 = before_colon[slash_pos + 1..].parse().ok()?;

    let name = before_colon[..slash_pos].to_string();
    Some((name, arity, line))
}

pub fn import_function_locations(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut rows = Vec::new();

    for (module, functions) in &graph.function_locations {
        for (func_key, loc) in functions {
            // Parse name, arity, line from key (new format: "func/arity:line")
            let (name, arity, line) = parse_function_key(func_key).unwrap_or_else(|| {
                // Fallback: use loc.line if key parsing fails
                (func_key.clone(), 0, loc.line)
            });

            let source_file_absolute = loc.source_file_absolute.as_deref().unwrap_or("");
            let pattern = loc.pattern.as_deref().unwrap_or("");
            let guard = loc.guard.as_deref().unwrap_or("");
            let source_sha = loc.source_sha.as_deref().unwrap_or("");
            let ast_sha = loc.ast_sha.as_deref().unwrap_or("");
            let generated_by = loc.generated_by.as_deref().unwrap_or("");
            let macro_source = loc.macro_source.as_deref().unwrap_or("");

            rows.push(vec![
                DataValue::Str(project.into()),                           // project
                DataValue::Str(module.clone().into()),                    // module
                DataValue::Str(name.into()),                              // name
                DataValue::from(arity as i64),                            // arity
                DataValue::from(line as i64),                             // line
                DataValue::Str(loc.file.as_deref().unwrap_or("").into()), // file
                DataValue::Str(source_file_absolute.into()),              // source_file_absolute
                DataValue::from(loc.column.unwrap_or(0) as i64),          // column
                DataValue::Str(loc.kind.clone().into()),                  // kind
                DataValue::from(loc.start_line as i64),                   // start_line
                DataValue::from(loc.end_line as i64),                     // end_line
                DataValue::Str(pattern.into()),                           // pattern
                DataValue::Str(guard.into()),                             // guard
                DataValue::Str(source_sha.into()),                        // source_sha
                DataValue::Str(ast_sha.into()),                           // ast_sha
                DataValue::from(loc.complexity as i64),                   // complexity
                DataValue::from(loc.max_nesting_depth as i64),            // max_nesting_depth
                DataValue::Str(generated_by.into()),                      // generated_by
                DataValue::Str(macro_source.into()),                      // macro_source
            ]);
        }
    }

    db.insert_rows(&FUNCTION_LOCATIONS, rows)
}

pub fn import_specs(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut rows = Vec::new();

    for (module, specs) in &graph.specs {
        for spec in specs {
            // Use first clause only (as per ticket recommendation)
            let (inputs_string, return_string, full) = spec
                .clauses
                .first()
                .map(|c| {
                    (
                        c.inputs_string.join(", "),
                        c.return_string.clone(),
                        c.full.clone(),
                    )
                })
                .unwrap_or_default();

            rows.push(vec![
                DataValue::Str(project.into()),           // project
                DataValue::Str(module.clone().into()),    // module
                DataValue::Str(spec.name.clone().into()), // name
                DataValue::from(spec.arity as i64),       // arity
                DataValue::Str(spec.kind.clone().into()), // kind
                DataValue::from(spec.line as i64),        // line
                DataValue::Str(inputs_string.into()),     // inputs_string
                DataValue::Str(return_string.into()),     // return_string
                DataValue::Str(full.into()),              // full
            ]);
        }
    }

    db.insert_rows(&SPECS, rows)
}

pub fn import_types(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut rows = Vec::new();

    for (module, types) in &graph.types {
        for type_def in types {
            let params = type_def.params.join(", ");

            rows.push(vec![
                DataValue::Str(project.into()),                     // project
                DataValue::Str(module.clone().into()),              // module
                DataValue::Str(type_def.name.clone().into()),       // name
                DataValue::Str(type_def.kind.clone().into()),       // kind
                DataValue::Str(params.into()),                      // params
                DataValue::from(type_def.line as i64),              // line
                DataValue::Str(type_def.definition.clone().into()), // definition
            ]);
        }
    }

    db.insert_rows(&TYPES, rows)
}

/// Import a parsed CallGraph into the database.
///
/// Imports all data (modules, functions, calls, structs, locations).
/// Schema is created automatically when the database connects via run_migrations().
/// This is the core import logic used by both the CLI command and test utilities.
pub fn import_graph(
    db: &dyn DatabaseBackend,
    project: &str,
    graph: &CallGraph,
) -> Result<ImportResult, Box<dyn Error>> {
    let mut result = ImportResult::default();

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
#[cfg(test)]
pub fn import_json_str(
    db: &dyn DatabaseBackend,
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
    use crate::db::open_mem_db;

    // Test deserialization with all new fields present
    #[test]
    fn test_function_location_deserialize_with_new_fields() {
        let json = r#"{
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

    // Test deserialization without new fields (backward compatibility)
    #[test]
    fn test_function_location_deserialize_without_new_fields() {
        let json = r#"{
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

        let backend = open_mem_db(true).expect("Failed to open db");

        let result =
            import_json_str(backend.as_ref(), json, "test_project").expect("Import should succeed");

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
}
