use std::error::Error;
use std::fs;
use std::path::Path;

use cozo::{DataValue, DbInstance};
use serde::Serialize;
use thiserror::Error;

use super::models::CallGraph;
use super::ImportCmd;
use crate::commands::Execute;
use crate::db::{escape_string, open_db, run_query, run_query_no_params, try_create_relation, Params};

/// Chunk size for batch database imports
const IMPORT_CHUNK_SIZE: usize = 500;

#[derive(Error, Debug)]
enum ImportError {
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

impl Execute for ImportCmd {
    type Output = ImportResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let mut result = ImportResult::default();

        // Read and parse call graph
        let content = fs::read_to_string(&self.file).map_err(|e| ImportError::FileReadFailed {
            path: self.file.display().to_string(),
            message: e.to_string(),
        })?;

        let graph: CallGraph =
            serde_json::from_str(&content).map_err(|e| ImportError::JsonParseFailed {
                message: e.to_string(),
            })?;

        let db = open_db(db_path)?;

        // Step 1: Create schemas
        result.schemas = create_schema(&db)?;

        // Step 2: Clear existing data if requested
        if self.clear {
            clear_project_data(&db, &self.project)?;
            result.cleared = true;
        }

        // Step 3: Import data
        result.modules_imported = import_modules(&db, &self.project, &graph)?;
        result.functions_imported = import_functions(&db, &self.project, &graph)?;
        result.calls_imported = import_calls(&db, &self.project, &graph)?;
        result.structs_imported = import_structs(&db, &self.project, &graph)?;
        result.function_locations_imported = import_function_locations(&db, &self.project, &graph)?;

        Ok(result)
    }
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

fn create_schema(db: &DbInstance) -> Result<SchemaResult, Box<dyn Error>> {
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

fn clear_project_data(db: &DbInstance, project: &str) -> Result<(), Box<dyn Error>> {
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

fn import_modules(
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

fn import_functions(
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

fn import_calls(
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

fn import_structs(
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

fn import_function_locations(
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_call_graph_json() -> &'static str {
        r#"{
            "structs": {
                "MyApp.User": {
                    "fields": [
                        {"default": "nil", "field": "name", "required": true, "inferred_type": "binary()"},
                        {"default": "0", "field": "age", "required": false, "inferred_type": "integer()"}
                    ]
                }
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1": {
                        "arity": 1,
                        "name": "get_user",
                        "file": "lib/my_app/accounts.ex",
                        "column": 7,
                        "kind": "def",
                        "end_line": 15,
                        "start_line": 10
                    }
                }
            },
            "calls": [
                {
                    "caller": {
                        "function": "get_user/1",
                        "line": 12,
                        "module": "MyApp.Accounts",
                        "file": "lib/my_app/accounts.ex",
                        "column": 5
                    },
                    "type": "remote",
                    "callee": {
                        "arity": 2,
                        "function": "get",
                        "module": "MyApp.Repo"
                    }
                }
            ],
            "type_signatures": {
                "MyApp.Accounts": {
                    "get_user/1": {
                        "arity": 1,
                        "name": "get_user",
                        "clauses": [
                            {"return": "dynamic()", "args": ["integer()"]}
                        ]
                    }
                }
            }
        }"#
    }

    fn create_temp_json_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write temp file");
        file
    }

    #[fixture]
    fn json_file() -> NamedTempFile {
        create_temp_json_file(sample_call_graph_json())
    }

    #[fixture]
    fn db_file() -> NamedTempFile {
        NamedTempFile::new().expect("Failed to create temp db file")
    }

    #[fixture]
    fn import_result(json_file: NamedTempFile, db_file: NamedTempFile) -> ImportResult {
        let cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };
        cmd.execute(db_file.path()).expect("Import should succeed")
    }

    #[rstest]
    fn test_import_creates_schemas(import_result: ImportResult) {
        assert!(!import_result.schemas.created.is_empty() || !import_result.schemas.already_existed.is_empty());
    }

    #[rstest]
    fn test_import_modules(import_result: ImportResult) {
        assert_eq!(import_result.modules_imported, 1); // MyApp.Accounts
    }

    #[rstest]
    fn test_import_functions(import_result: ImportResult) {
        assert_eq!(import_result.functions_imported, 1); // get_user/1
    }

    #[rstest]
    fn test_import_calls(import_result: ImportResult) {
        assert_eq!(import_result.calls_imported, 1);
    }

    #[rstest]
    fn test_import_structs(import_result: ImportResult) {
        assert_eq!(import_result.structs_imported, 2); // 2 fields in MyApp.User
    }

    #[rstest]
    fn test_import_function_locations(import_result: ImportResult) {
        assert_eq!(import_result.function_locations_imported, 1);
    }

    #[rstest]
    fn test_import_with_clear_flag(json_file: NamedTempFile, db_file: NamedTempFile) {
        // First import
        let cmd1 = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };
        cmd1.execute(db_file.path())
            .expect("First import should succeed");

        // Second import with clear
        let cmd2 = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: true,
        };
        let result = cmd2
            .execute(db_file.path())
            .expect("Second import should succeed");

        assert!(result.cleared);
        assert_eq!(result.modules_imported, 1);
    }

    #[rstest]
    fn test_import_empty_graph(db_file: NamedTempFile) {
        let empty_json = r#"{
            "structs": {},
            "function_locations": {},
            "calls": [],
            "type_signatures": {}
        }"#;

        let json_file = create_temp_json_file(empty_json);

        let cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };

        let result = cmd.execute(db_file.path()).expect("Import should succeed");

        assert_eq!(result.modules_imported, 0);
        assert_eq!(result.functions_imported, 0);
        assert_eq!(result.calls_imported, 0);
        assert_eq!(result.structs_imported, 0);
        assert_eq!(result.function_locations_imported, 0);
    }

    #[rstest]
    fn test_import_invalid_json_fails(db_file: NamedTempFile) {
        let invalid_json = "{ not valid json }";
        let json_file = create_temp_json_file(invalid_json);

        let cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };

        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }

    #[rstest]
    fn test_import_nonexistent_file_fails(db_file: NamedTempFile) {
        let cmd = ImportCmd {
            file: "/nonexistent/path/call_graph.json".into(),
            project: "test_project".to_string(),
            clear: false,
        };

        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
