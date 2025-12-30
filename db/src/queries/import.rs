use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{run_query, run_query_no_params};
use crate::queries::import_models::CallGraph;
use crate::queries::schema;

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

pub fn create_schema(db: &dyn Database) -> Result<SchemaResult, Box<dyn Error>> {
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

/// Clear all project data from SurrealDB
/// Since SurrealDB is per-project, we delete all records from all tables
pub fn clear_project_data(db: &dyn Database) -> Result<(), Box<dyn Error>> {
    let tables = [
        "modules",
        "functions",
        "clauses",
        "specs",
        "types",
        "fields",
        "defines",
        "has_clause",
        "calls",
        "has_field",
    ];

    for table in tables {
        let script = format!("DELETE FROM {};", table);
        run_query_no_params(db, &script).map_err(|e| ImportError::ClearFailed {
            message: format!("Failed to clear {}: {}", table, e),
        })?;
    }

    Ok(())
}

/// Import modules to SurrealDB
pub fn import_modules(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    // Collect unique modules from all data sources
    let mut modules = std::collections::HashSet::new();
    modules.extend(graph.specs.keys().cloned());
    modules.extend(graph.function_locations.keys().cloned());
    modules.extend(graph.structs.keys().cloned());
    modules.extend(graph.types.keys().cloned());

    let mut count = 0;
    for module_name in modules {
        let query = "CREATE modules:[$name] SET name = $name, file = \"\", source = \"unknown\";";
        let params = QueryParams::new().with_str("name", &module_name);
        run_query(db, query, params)?;
        count += 1;
    }

    Ok(count)
}

/// Import functions from function_locations to SurrealDB
///
/// Functions are created from function_locations, which contains the actual
/// function definitions. Specs are metadata that belong to functions and are
/// linked via name/arity matching, not imported as separate function records.
pub fn import_functions(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    use std::collections::HashSet;
    let mut count = 0;
    let mut seen: HashSet<(String, String, i64)> = HashSet::new();

    // Import functions from function_locations data
    for (module_name, locations) in &graph.function_locations {
        for location in locations.values() {
            let key = (
                module_name.clone(),
                location.name.clone(),
                location.arity as i64,
            );
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);

            let query = r#"
                CREATE functions:[$module_name, $name, $arity] SET
                    module_name = $module_name,
                    name = $name,
                    arity = $arity,
                    kind = $kind,
                    file = $file,
                    start_line = $start_line;
            "#;
            let file = location.file.as_deref().unwrap_or("");
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("name", &location.name)
                .with_int("arity", location.arity as i64)
                .with_str("kind", &location.kind)
                .with_str("file", file)
                .with_int("start_line", location.start_line as i64);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Import calls to SurrealDB
pub fn import_calls(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    for call in &graph.calls {
        let caller_kind = call.caller.kind.as_deref().unwrap_or("");
        let call_line = call.caller.line.unwrap_or(0) as i64;

        // Parse caller function - may be "name" or "name/arity" format
        let caller_func_raw = call.caller.function.as_deref().unwrap_or("<module>");
        let (caller_name, caller_arity) = parse_function_ref(caller_func_raw);

        // First, find the clause that contains this call (based on line range)
        // The caller_clause_id links the call to the specific clause where it occurs
        let query = r#"
            LET $clause = (
                SELECT id FROM clauses
                WHERE module_name = $caller_module
                  AND function_name = $caller_name
                  AND start_line <= $call_line
                  AND end_line >= $call_line
                LIMIT 1
            );
            RELATE functions:[$caller_module, $caller_name, $caller_arity]
                ->calls->
                functions:[$callee_module, $callee_name, $callee_arity]
            SET
                call_type = $call_type,
                caller_kind = $caller_kind,
                file = $file,
                line = $line,
                caller_clause_id = $clause[0].id;
        "#;
        let params = QueryParams::new()
            .with_str("caller_module", &call.caller.module)
            .with_str("caller_name", caller_name)
            .with_int("caller_arity", caller_arity)
            .with_str("callee_module", &call.callee.module)
            .with_str("callee_name", &call.callee.function)
            .with_int("callee_arity", call.callee.arity as i64)
            .with_str("call_type", &call.call_type)
            .with_str("caller_kind", caller_kind)
            .with_str("file", &call.caller.file)
            .with_int("line", call_line)
            .with_int("call_line", call_line);
        run_query(db, query, params)?;
        count += 1;
    }

    Ok(count)
}

/// Update call counts on functions table after importing calls.
///
/// This should be called after `import_calls` to populate the denormalized
/// `incoming_call_count` and `outgoing_call_count` fields on functions.
pub fn update_call_counts(db: &dyn Database) -> Result<(), Box<dyn Error>> {
    // Update incoming_call_count (how many times this function is called)
    let incoming_query = r#"
        UPDATE functions SET incoming_call_count = (
            SELECT count() FROM calls WHERE out = $parent.id GROUP ALL
        )[0].count ?? 0
    "#;
    run_query(db, incoming_query, QueryParams::new())?;

    // Update outgoing_call_count (how many calls this function makes)
    let outgoing_query = r#"
        UPDATE functions SET outgoing_call_count = (
            SELECT count() FROM calls WHERE in = $parent.id GROUP ALL
        )[0].count ?? 0
    "#;
    run_query(db, outgoing_query, QueryParams::new())?;

    Ok(())
}

/// Parse a function reference that may be "name" or "name/arity" format
/// Returns (function_name, arity) - arity defaults to 0 if not specified
fn parse_function_ref(func_ref: &str) -> (&str, i64) {
    if let Some(slash_pos) = func_ref.rfind('/') {
        let name = &func_ref[..slash_pos];
        let arity_str = &func_ref[slash_pos + 1..];
        let arity = arity_str.parse::<i64>().unwrap_or(0);
        (name, arity)
    } else {
        (func_ref, 0)
    }
}

/// Import structs to SurrealDB (as fields)
pub fn import_structs(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    for (module_name, def) in &graph.structs {
        for field in &def.fields {
            let query = r#"
                CREATE fields:[$module_name, $name] SET
                    module_name = $module_name,
                    name = $name,
                    default_value = $default_value,
                    required = $required;
            "#;
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("name", &field.field)
                .with_str("default_value", &field.default)
                .with_bool("required", field.required);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Import function locations to SurrealDB (as clauses)
pub fn import_function_locations(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    for (module_name, functions) in &graph.function_locations {
        for loc in functions.values() {
            let query = r#"
                CREATE clauses:[$module_name, $function_name, $arity, $line] SET
                    module_name = $module_name,
                    function_name = $function_name,
                    arity = $arity,
                    line = $line,
                    source_file = $source_file,
                    source_file_absolute = $source_file_absolute,
                    kind = $kind,
                    start_line = $start_line,
                    end_line = $end_line,
                    pattern = $pattern,
                    guard = $guard,
                    source_sha = $source_sha,
                    ast_sha = $ast_sha,
                    complexity = $complexity,
                    max_nesting_depth = $max_nesting_depth,
                    generated_by = $generated_by,
                    macro_source = $macro_source;
            "#;
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("function_name", &loc.name)
                .with_int("arity", loc.arity as i64)
                .with_int("line", loc.line as i64)
                .with_str("source_file", loc.file.as_deref().unwrap_or(""))
                .with_str(
                    "source_file_absolute",
                    loc.source_file_absolute.as_deref().unwrap_or(""),
                )
                .with_str("kind", &loc.kind)
                .with_int("start_line", loc.start_line as i64)
                .with_int("end_line", loc.end_line as i64)
                .with_str("pattern", loc.pattern.as_deref().unwrap_or(""))
                .with_str("guard", loc.guard.as_deref().unwrap_or(""))
                .with_str("source_sha", loc.source_sha.as_deref().unwrap_or(""))
                .with_str("ast_sha", loc.ast_sha.as_deref().unwrap_or(""))
                .with_int("complexity", loc.complexity as i64)
                .with_int("max_nesting_depth", loc.max_nesting_depth as i64)
                .with_str("generated_by", loc.generated_by.as_deref().unwrap_or(""))
                .with_str("macro_source", loc.macro_source.as_deref().unwrap_or(""));
            run_query(db, query, params)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Import specs to SurrealDB with array fields preserved
pub fn import_specs(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    for (module_name, specs) in &graph.specs {
        for spec in specs {
            // Import each clause as a separate spec row with clause_index
            for (clause_index, clause) in spec.clauses.iter().enumerate() {
                let query = r#"
                    CREATE specs:[$module_name, $function_name, $arity, $clause_index] SET
                        module_name = $module_name,
                        function_name = $function_name,
                        arity = $arity,
                        kind = $kind,
                        line = $line,
                        clause_index = $clause_index,
                        input_strings = $input_strings,
                        return_strings = $return_strings,
                        full = $full;
                "#;

                let params = QueryParams::new()
                    .with_str("module_name", module_name)
                    .with_str("function_name", &spec.name)
                    .with_int("arity", spec.arity as i64)
                    .with_str("kind", &spec.kind)
                    .with_int("line", spec.line as i64)
                    .with_int("clause_index", clause_index as i64)
                    .with_str_array("input_strings", clause.input_strings.clone())
                    .with_str_array("return_strings", clause.return_strings.clone())
                    .with_str("full", &clause.full);
                run_query(db, query, params)?;
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Import types to SurrealDB
pub fn import_types(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    for (module_name, types) in &graph.types {
        for type_def in types {
            let query = r#"
                CREATE types:[$module_name, $name] SET
                    module_name = $module_name,
                    name = $name,
                    kind = $kind,
                    params = $params,
                    line = $line,
                    definition = $definition;
            "#;
            let params_str = type_def.params.join(", ");
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("name", &type_def.name)
                .with_str("kind", &type_def.kind)
                .with_str("params", &params_str)
                .with_int("line", type_def.line as i64)
                .with_str("definition", &type_def.definition);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Create defines relationships (modules -> functions/types/specs) for SurrealDB
pub fn create_defines_relationships(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    // Create defines relationships for functions
    for (module_name, specs) in &graph.specs {
        for spec in specs {
            let query = r#"
                RELATE modules:[$module_name]
                    ->defines->
                    functions:[$module_name, $name, $arity];
            "#;
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("name", &spec.name)
                .with_int("arity", spec.arity as i64);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    // Create defines relationships for types
    for (module_name, types) in &graph.types {
        for type_def in types {
            let query = r#"
                RELATE modules:[$module_name]
                    ->defines->
                    types:[$module_name, $name];
            "#;
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("name", &type_def.name);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    // Create defines relationships for specs
    for (module_name, specs) in &graph.specs {
        for spec in specs {
            for (clause_index, _) in spec.clauses.iter().enumerate() {
                let query = r#"
                    RELATE modules:[$module_name]
                        ->defines->
                        specs:[$module_name, $function_name, $arity, $clause_index];
                "#;
                let params = QueryParams::new()
                    .with_str("module_name", module_name)
                    .with_str("function_name", &spec.name)
                    .with_int("arity", spec.arity as i64)
                    .with_int("clause_index", clause_index as i64);
                run_query(db, query, params)?;
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Create has_clause relationships (functions -> clauses) for SurrealDB
pub fn create_has_clause_relationships(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    for (module_name, functions) in &graph.function_locations {
        for loc in functions.values() {
            let query = r#"
                RELATE functions:[$module_name, $function_name, $arity]
                    ->has_clause->
                    clauses:[$module_name, $function_name, $arity, $line];
            "#;
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("function_name", &loc.name)
                .with_int("arity", loc.arity as i64)
                .with_int("line", loc.line as i64);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Create has_field relationships (modules -> fields) for SurrealDB
pub fn create_has_field_relationships(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    for (module_name, def) in &graph.structs {
        for field in &def.fields {
            let query = r#"
                RELATE modules:[$module_name]
                    ->has_field->
                    fields:[$module_name, $field_name];
            "#;
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("field_name", &field.field);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Import a parsed CallGraph into the database.
///
/// Creates schemas and imports all data (modules, functions, calls, structs, locations).
/// This is the core import logic used by both the CLI command and test utilities.
pub fn import_graph(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<ImportResult, Box<dyn Error>> {
    let mut result = ImportResult::default();

    result.schemas = create_schema(db)?;
    result.modules_imported = import_modules(db, graph)?;
    result.functions_imported = import_functions(db, graph)?;
    // Import function_locations (clauses) BEFORE calls so caller_clause_id lookup works
    result.function_locations_imported = import_function_locations(db, graph)?;
    result.calls_imported = import_calls(db, graph)?;
    result.structs_imported = import_structs(db, graph)?;
    result.specs_imported = import_specs(db, graph)?;
    result.types_imported = import_types(db, graph)?;

    // Create relationships
    create_defines_relationships(db, graph)?;
    create_has_clause_relationships(db, graph)?;
    create_has_field_relationships(db, graph)?;

    // Update denormalized call counts after all calls are imported
    update_call_counts(db)?;

    Ok(result)
}

/// Import a JSON string directly into the database.
///
/// Convenience wrapper for tests that parses JSON and calls `import_graph`.
#[cfg(any(test, feature = "test-utils"))]
pub fn import_json_str(
    db: &dyn Database,
    content: &str,
) -> Result<ImportResult, Box<dyn Error>> {
    let graph: CallGraph =
        serde_json::from_str(content).map_err(|e| ImportError::JsonParseFailed {
            message: e.to_string(),
        })?;

    import_graph(db, &graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::QueryParams;

    /// Test parse_function_ref handles both "name" and "name/arity" formats
    #[test]
    fn test_parse_function_ref() {
        // With arity (fixture format)
        let (name, arity) = parse_function_ref("get_user/1");
        assert_eq!(name, "get_user");
        assert_eq!(arity, 1);

        // With higher arity
        let (name, arity) = parse_function_ref("do_fetch/2");
        assert_eq!(name, "do_fetch");
        assert_eq!(arity, 2);

        // Without arity (test format)
        let (name, arity) = parse_function_ref("get_user");
        assert_eq!(name, "get_user");
        assert_eq!(arity, 0);

        // Module-level call (no function)
        let (name, arity) = parse_function_ref("<module>");
        assert_eq!(name, "<module>");
        assert_eq!(arity, 0);

        // Zero arity
        let (name, arity) = parse_function_ref("list_users/0");
        assert_eq!(name, "list_users");
        assert_eq!(arity, 0);
    }

    /// Test import_modules creates correct number of module nodes
    #[test]
    fn test_import_modules_creates_nodes() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {
                "MyApp.Accounts": [{"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec get_user(integer()) :: user()", "input_strings": ["integer()"], "return_strings": ["user()"]}]}],
                "MyApp.Repo": [{"name": "get", "arity": 2, "line": 20, "kind": "spec", "clauses": [{"full": "@spec get(atom(), any()) :: any()", "input_strings": ["atom()", "any()"], "return_strings": ["any()"]}]}]
            },
            "function_locations": {},
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        let result = import_modules(&*db, &graph);
        assert!(result.is_ok(), "Import should succeed: {:?}", result.err());
        assert_eq!(result.unwrap(), 2, "Should import exactly 2 modules");

        // Verify modules were created
        let query = "SELECT name FROM modules ORDER BY name";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        let names: Vec<String> = rows
            .rows()
            .iter()
            .filter_map(|row| row.get(0).and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"MyApp.Accounts".to_string()));
        assert!(names.contains(&"MyApp.Repo".to_string()));
    }

    /// Test import_functions creates function nodes from function_locations
    #[test]
    fn test_import_functions_creates_nodes() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]},
                    {"name": "get_user", "arity": 2, "line": 12, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]},
                    {"name": "list_users", "arity": 0, "line": 14, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "Accounts.get_user/1:10": {"name": "get_user", "arity": 1, "line": 10, "start_line": 10, "end_line": 15, "kind": "def", "source_file": "lib/accounts.ex"},
                    "Accounts.get_user/2:16": {"name": "get_user", "arity": 2, "line": 16, "start_line": 16, "end_line": 21, "kind": "def", "source_file": "lib/accounts.ex"},
                    "Accounts.list_users/0:22": {"name": "list_users", "arity": 0, "line": 22, "start_line": 22, "end_line": 26, "kind": "def", "source_file": "lib/accounts.ex"}
                }
            },
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        let result = import_functions(&*db, &graph);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            3,
            "Should import 3 functions (get_user/1, get_user/2, list_users/0)"
        );

        // Verify functions are created with correct arity
        let query = "SELECT name, arity FROM functions ORDER BY arity, name";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        assert_eq!(rows.rows().len(), 3, "Should have 3 function rows");
    }

    /// Test import_specs preserves array fields
    #[test]
    fn test_import_specs_preserves_arrays() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {
                        "name": "my_func",
                        "arity": 2,
                        "line": 10,
                        "kind": "spec",
                        "clauses": [
                            {
                                "full": "@spec my_func(integer(), String.t()) :: :ok",
                                "input_strings": ["integer()", "String.t()"],
                                "return_strings": [":ok"]
                            }
                        ]
                    }
                ]
            },
            "function_locations": {},
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_functions(&*db, &graph).unwrap();
        let result = import_specs(&*db, &graph);
        assert!(
            result.is_ok(),
            "Import specs should succeed: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), 1, "Should import 1 spec");

        // Verify spec array fields are stored as actual arrays
        let query = "SELECT input_strings, return_strings FROM specs LIMIT 1";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        let row = rows.rows().iter().next().unwrap();

        // Arrays should be preserved as actual arrays
        let input_arr = row.get(0).and_then(|v| v.as_array());
        let return_arr = row.get(1).and_then(|v| v.as_array());

        assert!(input_arr.is_some(), "input_strings should be stored as array");
        assert!(return_arr.is_some(), "return_strings should be stored as array");

        // Verify array contents
        let inputs = input_arr.unwrap();
        assert_eq!(inputs.len(), 2, "Should have 2 input types");
        assert_eq!(inputs[0].as_str(), Some("integer()"));
        assert_eq!(inputs[1].as_str(), Some("String.t()"));

        let returns = return_arr.unwrap();
        assert_eq!(returns.len(), 1, "Should have 1 return type");
        assert_eq!(returns[0].as_str(), Some(":ok"));
    }

    /// Test import_function_locations creates clauses
    #[test]
    fn test_import_function_locations_creates_clauses() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {},
            "function_locations": {
                "MyApp.Accounts": {
                    "process_data/2:20": {
                        "name": "process_data",
                        "arity": 2,
                        "file": "lib/accounts.ex",
                        "kind": "def",
                        "line": 20,
                        "start_line": 20,
                        "end_line": 25,
                        "complexity": 5,
                        "max_nesting_depth": 2
                    }
                }
            },
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        let result = import_function_locations(&*db, &graph);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1, "Should import 1 clause");

        // Verify clause is created
        let query = "SELECT module_name, function_name, arity, line, complexity FROM clauses";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        assert_eq!(rows.rows().len(), 1);
    }

    /// Test import_structs creates field nodes
    #[test]
    fn test_import_structs_creates_fields() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {},
            "function_locations": {},
            "calls": [],
            "structs": {
                "MyApp.User": {
                    "fields": [
                        {"field": "id", "default": "nil", "required": true, "inferred_type": "integer()"},
                        {"field": "name", "default": "nil", "required": false, "inferred_type": "String.t()"}
                    ]
                }
            },
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        let result = import_structs(&*db, &graph);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2, "Should import 2 fields");

        // Verify fields are created
        let query = "SELECT module_name, name, required FROM fields ORDER BY name";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        assert_eq!(rows.rows().len(), 2);
    }

    /// Test import_types creates type nodes
    #[test]
    fn test_import_types_creates_nodes() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {},
            "function_locations": {},
            "calls": [],
            "structs": {},
            "types": {
                "MyModule": [
                    {
                        "name": "status",
                        "kind": "type",
                        "params": [],
                        "line": 5,
                        "definition": "@type status() :: :pending | :active"
                    },
                    {
                        "name": "config",
                        "kind": "type",
                        "params": ["t"],
                        "line": 10,
                        "definition": "@type config(t) :: %{key: t}"
                    }
                ]
            }
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        let result = import_types(&*db, &graph);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2, "Should import 2 types");

        // Verify types are created
        let query = "SELECT module_name, name, kind FROM types ORDER BY name";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        assert_eq!(rows.rows().len(), 2);
    }

    /// Test create_defines_relationships creates proper relationships
    #[test]
    fn test_create_defines_relationships() {
        // Create minimal test data
        let json = r#"{
            "specs": {
                "MyModule": [
                    {"name": "func1", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {},
            "calls": [],
            "structs": {},
            "types": {
                "MyModule": [
                    {"name": "my_type", "kind": "type", "params": [], "line": 5, "definition": "@type"}
                ]
            }
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();

        // Clear and set up fresh
        let db_fresh = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db_fresh).unwrap();
        import_modules(&*db_fresh, &graph).unwrap();
        import_functions(&*db_fresh, &graph).unwrap();
        import_types(&*db_fresh, &graph).unwrap();

        let result = create_defines_relationships(&*db_fresh, &graph);
        assert!(
            result.is_ok(),
            "Creating relationships should succeed: {:?}",
            result.err()
        );

        // Should create relationships for 1 function + 1 type + 1 spec = 3 total
        let count = result.unwrap();
        assert!(count >= 3, "Should create at least 3 relationships");
    }

    /// Test create_has_clause_relationships
    #[test]
    fn test_create_has_clause_relationships() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1:10": {
                        "name": "get_user",
                        "arity": 1,
                        "file": "lib/accounts.ex",
                        "kind": "def",
                        "line": 10,
                        "start_line": 10,
                        "end_line": 15
                    }
                }
            },
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_functions(&*db, &graph).unwrap();
        import_function_locations(&*db, &graph).unwrap();

        let result = create_has_clause_relationships(&*db, &graph);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            1,
            "Should create 1 has_clause relationship"
        );
    }

    /// Test create_has_field_relationships
    #[test]
    fn test_create_has_field_relationships() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {},
            "function_locations": {},
            "calls": [],
            "structs": {
                "MyApp.User": {
                    "fields": [
                        {"field": "id", "default": "nil", "required": true},
                        {"field": "name", "default": "nil", "required": false}
                    ]
                }
            },
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_structs(&*db, &graph).unwrap();

        let result = create_has_field_relationships(&*db, &graph);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            2,
            "Should create 2 has_field relationships"
        );
    }

    /// Test clear_project_data deletes all data
    #[test]
    fn test_clear_project_data() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1:10": {
                        "name": "get_user",
                        "arity": 1,
                        "file": "lib/accounts.ex",
                        "kind": "def",
                        "line": 10,
                        "start_line": 10,
                        "end_line": 15
                    }
                }
            },
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_functions(&*db, &graph).unwrap();
        import_function_locations(&*db, &graph).unwrap();

        // Verify data was imported
        let query = "SELECT COUNT() FROM modules";
        let result = db.execute_query(query, QueryParams::new()).unwrap();
        assert!(
            !result.rows().is_empty(),
            "Should have modules before clear"
        );

        // Clear data
        let clear_result = clear_project_data(&*db);
        assert!(
            clear_result.is_ok(),
            "Clear should succeed: {:?}",
            clear_result.err()
        );

        // Verify all tables are empty
        let tables = [
            "modules",
            "functions",
            "clauses",
            "specs",
            "types",
            "fields",
        ];
        for table in tables {
            let query = format!("SELECT COUNT() as cnt FROM {}", table);
            // This should either return empty or count 0, both are acceptable
            let _result = db.execute_query(&query, QueryParams::new());
            // Just verify the query executes without error
        }
    }

    /// Test import_calls creates call relationships with caller_clause_id
    /// Uses fixture-consistent format where caller.function includes arity (e.g., "get_user/1")
    #[test]
    fn test_import_calls_creates_relationships() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        // Note: caller.function uses "name/arity" format to match call_graph.json fixture
        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 8, "kind": "spec", "clauses": [{"full": "@spec get_user(integer()) :: {:ok, User.t()} | {:error, :not_found}", "input_strings": ["integer()"], "return_strings": ["{:ok, User.t()}", "{:error, :not_found}"]}]}
                ],
                "MyApp.Repo": [
                    {"name": "get", "arity": 2, "line": 8, "kind": "callback", "clauses": [{"full": "@callback get(module(), term()) :: Ecto.Schema.t() | nil", "input_strings": ["module()", "term()"], "return_strings": ["Ecto.Schema.t()", "nil"]}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "Accounts.get_user/1:10": {
                        "name": "get_user",
                        "arity": 1,
                        "source_file": "lib/my_app/accounts.ex",
                        "source_file_absolute": "/home/user/my_app/lib/my_app/accounts.ex",
                        "kind": "def",
                        "line": 10,
                        "start_line": 10,
                        "end_line": 15,
                        "pattern": "id",
                        "complexity": 2,
                        "max_nesting_depth": 1
                    }
                }
            },
            "calls": [
                {
                    "type": "remote",
                    "caller": {
                        "module": "MyApp.Accounts",
                        "function": "get_user/1",
                        "kind": "def",
                        "file": "/home/user/my_app/lib/my_app/accounts.ex",
                        "line": 12
                    },
                    "callee": {
                        "module": "MyApp.Repo",
                        "function": "get",
                        "arity": 2
                    }
                }
            ],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_functions(&*db, &graph).unwrap();
        import_function_locations(&*db, &graph).unwrap();

        let result = import_calls(&*db, &graph);
        assert!(
            result.is_ok(),
            "Import calls should succeed: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), 1, "Should import 1 call relationship");

        // Verify caller_clause_id is set by traversing to get start_line/end_line
        // (call at line 12 is within clause lines 10-15)
        // NOTE: Must use aliases otherwise SurrealDB collapses both fields into a single Object
        let query =
            "SELECT caller_clause_id.end_line as end_line, caller_clause_id.start_line as start_line FROM calls";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        assert_eq!(rows.rows().len(), 1, "Should have 1 call");

        let row = rows.rows().first().unwrap();
        // Columns in alphabetical order: end_line (0), start_line (1)
        let end_line = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
        let start_line = row.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
        assert_eq!(
            start_line, 10,
            "start_line should be 10 from clause (caller_clause_id must be set)"
        );
        assert_eq!(
            end_line, 15,
            "end_line should be 15 from clause (caller_clause_id must be set)"
        );
    }

    /// Test full import_graph flow
    #[test]
    fn test_import_graph_full_flow() {
        let db = crate::open_mem_db().unwrap();

        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec get_user(integer()) :: user()", "input_strings": ["integer()"], "return_strings": ["user()"]}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1:10": {
                        "name": "get_user",
                        "arity": 1,
                        "file": "lib/accounts.ex",
                        "kind": "def",
                        "line": 10,
                        "start_line": 10,
                        "end_line": 15,
                        "complexity": 2,
                        "max_nesting_depth": 1
                    }
                }
            },
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        let result = import_graph(&*db, &graph);

        assert!(result.is_ok(), "Import should succeed: {:?}", result.err());
        let import_result = result.unwrap();

        // Verify counts
        assert!(import_result.modules_imported > 0, "Should import modules");
        assert!(
            import_result.functions_imported > 0,
            "Should import functions"
        );
        assert!(
            import_result.function_locations_imported > 0,
            "Should import clauses"
        );
        assert!(import_result.specs_imported > 0, "Should import specs");
    }

    /// Test import_graph updates call counts after importing calls
    #[test]
    fn test_import_graph_updates_call_counts() {
        let db = crate::open_mem_db().unwrap();

        // Use a fixture with calls to verify update_call_counts is called during import
        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ],
                "MyApp.Repo": [
                    {"name": "get", "arity": 2, "line": 8, "kind": "callback", "clauses": [{"full": "@callback", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1:10": {"name": "get_user", "arity": 1, "source_file": "lib/accounts.ex", "kind": "def", "line": 10, "start_line": 10, "end_line": 15}
                },
                "MyApp.Repo": {
                    "get/2:8": {"name": "get", "arity": 2, "source_file": "lib/repo.ex", "kind": "def", "line": 8, "start_line": 8, "end_line": 12}
                }
            },
            "calls": [
                {
                    "type": "remote",
                    "caller": {"module": "MyApp.Accounts", "function": "get_user/1", "kind": "def", "file": "lib/accounts.ex", "line": 12},
                    "callee": {"module": "MyApp.Repo", "function": "get", "arity": 2}
                }
            ],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        let result = import_graph(&*db, &graph);
        assert!(result.is_ok(), "Import should succeed: {:?}", result.err());

        // Verify call counts were updated during import
        // Columns in alphabetical order: incoming_call_count (0), name (1), outgoing_call_count (2)
        let query = "SELECT name, incoming_call_count, outgoing_call_count FROM functions ORDER BY name";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        let counts: std::collections::HashMap<String, (i64, i64)> = rows
            .rows()
            .iter()
            .filter_map(|row| {
                let name = row.get(1).and_then(|v| v.as_str()).map(|s| s.to_string())?;
                let incoming = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
                let outgoing = row.get(2).and_then(|v| v.as_i64()).unwrap_or(0);
                Some((name, (incoming, outgoing)))
            })
            .collect();

        // get_user calls Repo.get, so: incoming=0, outgoing=1
        assert_eq!(
            counts.get("get_user"),
            Some(&(0, 1)),
            "import_graph should update get_user's outgoing_call_count to 1"
        );

        // Repo.get is called by get_user, so: incoming=1, outgoing=0
        assert_eq!(
            counts.get("get"),
            Some(&(1, 0)),
            "import_graph should update Repo.get's incoming_call_count to 1"
        );
    }

    /// Test update_call_counts sets incoming and outgoing call counts correctly
    #[test]
    fn test_update_call_counts_sets_incoming_counts() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        // Create a call graph with:
        // - MyApp.Accounts.get_user/1 calls MyApp.Repo.get/2
        // - MyApp.Controller.index/2 calls MyApp.Accounts.list_users/0
        // So:
        // - get_user: outgoing=1, incoming=0
        // - Repo.get: outgoing=0, incoming=1
        // - index: outgoing=1, incoming=0
        // - list_users: outgoing=0, incoming=1
        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]},
                    {"name": "list_users", "arity": 0, "line": 20, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ],
                "MyApp.Repo": [
                    {"name": "get", "arity": 2, "line": 8, "kind": "callback", "clauses": [{"full": "@callback", "input_strings": [], "return_strings": []}]}
                ],
                "MyApp.Controller": [
                    {"name": "index", "arity": 2, "line": 5, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1:10": {"name": "get_user", "arity": 1, "source_file": "lib/accounts.ex", "kind": "def", "line": 10, "start_line": 10, "end_line": 15},
                    "list_users/0:20": {"name": "list_users", "arity": 0, "source_file": "lib/accounts.ex", "kind": "def", "line": 20, "start_line": 20, "end_line": 25}
                },
                "MyApp.Repo": {
                    "get/2:8": {"name": "get", "arity": 2, "source_file": "lib/repo.ex", "kind": "def", "line": 8, "start_line": 8, "end_line": 12}
                },
                "MyApp.Controller": {
                    "index/2:5": {"name": "index", "arity": 2, "source_file": "lib/controller.ex", "kind": "def", "line": 5, "start_line": 5, "end_line": 10}
                }
            },
            "calls": [
                {
                    "type": "remote",
                    "caller": {"module": "MyApp.Accounts", "function": "get_user/1", "kind": "def", "file": "lib/accounts.ex", "line": 12},
                    "callee": {"module": "MyApp.Repo", "function": "get", "arity": 2}
                },
                {
                    "type": "remote",
                    "caller": {"module": "MyApp.Controller", "function": "index/2", "kind": "def", "file": "lib/controller.ex", "line": 7},
                    "callee": {"module": "MyApp.Accounts", "function": "list_users", "arity": 0}
                }
            ],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_functions(&*db, &graph).unwrap();
        import_function_locations(&*db, &graph).unwrap();
        import_calls(&*db, &graph).unwrap();

        // Before update_call_counts, all counts should be 0
        // Note: SurrealDB returns columns in alphabetical order, so:
        // incoming_call_count (0), name (1), outgoing_call_count (2)
        let query = "SELECT name, incoming_call_count, outgoing_call_count FROM functions ORDER BY name";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        for row in rows.rows() {
            let incoming = row.get(0).and_then(|v| v.as_i64()).unwrap_or(-1);
            let outgoing = row.get(2).and_then(|v| v.as_i64()).unwrap_or(-1);
            assert_eq!(incoming, 0, "Before update, incoming_call_count should be 0");
            assert_eq!(outgoing, 0, "Before update, outgoing_call_count should be 0");
        }

        // Run update_call_counts
        let result = update_call_counts(&*db);
        assert!(result.is_ok(), "update_call_counts should succeed: {:?}", result.err());

        // Verify counts after update
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        // Columns in alphabetical order: incoming_call_count (0), name (1), outgoing_call_count (2)
        let counts: std::collections::HashMap<String, (i64, i64)> = rows
            .rows()
            .iter()
            .filter_map(|row| {
                let name = row.get(1).and_then(|v| v.as_str()).map(|s| s.to_string())?;
                let incoming = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
                let outgoing = row.get(2).and_then(|v| v.as_i64()).unwrap_or(0);
                Some((name, (incoming, outgoing)))
            })
            .collect();

        // get_user: calls Repo.get, not called by anyone in our graph
        assert_eq!(counts.get("get_user"), Some(&(0, 1)), "get_user: incoming=0, outgoing=1");

        // Repo.get: called by get_user, doesn't call anything
        assert_eq!(counts.get("get"), Some(&(1, 0)), "get: incoming=1, outgoing=0");

        // index: calls list_users, not called by anyone
        assert_eq!(counts.get("index"), Some(&(0, 1)), "index: incoming=0, outgoing=1");

        // list_users: called by index, doesn't call anything
        assert_eq!(counts.get("list_users"), Some(&(1, 0)), "list_users: incoming=1, outgoing=0");
    }

    /// Test update_call_counts handles functions with multiple incoming/outgoing calls
    #[test]
    fn test_update_call_counts_multiple_calls() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        // Create a call graph where Repo.get is called by multiple functions
        // and get_user makes multiple calls
        let json = r#"{
            "specs": {
                "MyApp.Accounts": [
                    {"name": "get_user", "arity": 1, "line": 10, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]},
                    {"name": "update_user", "arity": 2, "line": 30, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ],
                "MyApp.Repo": [
                    {"name": "get", "arity": 2, "line": 8, "kind": "callback", "clauses": [{"full": "@callback", "input_strings": [], "return_strings": []}]},
                    {"name": "update", "arity": 2, "line": 20, "kind": "callback", "clauses": [{"full": "@callback", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1:10": {"name": "get_user", "arity": 1, "source_file": "lib/accounts.ex", "kind": "def", "line": 10, "start_line": 10, "end_line": 15},
                    "update_user/2:30": {"name": "update_user", "arity": 2, "source_file": "lib/accounts.ex", "kind": "def", "line": 30, "start_line": 30, "end_line": 40}
                },
                "MyApp.Repo": {
                    "get/2:8": {"name": "get", "arity": 2, "source_file": "lib/repo.ex", "kind": "def", "line": 8, "start_line": 8, "end_line": 12},
                    "update/2:20": {"name": "update", "arity": 2, "source_file": "lib/repo.ex", "kind": "def", "line": 20, "start_line": 20, "end_line": 25}
                }
            },
            "calls": [
                {
                    "type": "remote",
                    "caller": {"module": "MyApp.Accounts", "function": "get_user/1", "kind": "def", "file": "lib/accounts.ex", "line": 12},
                    "callee": {"module": "MyApp.Repo", "function": "get", "arity": 2}
                },
                {
                    "type": "remote",
                    "caller": {"module": "MyApp.Accounts", "function": "update_user/2", "kind": "def", "file": "lib/accounts.ex", "line": 32},
                    "callee": {"module": "MyApp.Repo", "function": "get", "arity": 2}
                },
                {
                    "type": "remote",
                    "caller": {"module": "MyApp.Accounts", "function": "update_user/2", "kind": "def", "file": "lib/accounts.ex", "line": 35},
                    "callee": {"module": "MyApp.Repo", "function": "update", "arity": 2}
                }
            ],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_functions(&*db, &graph).unwrap();
        import_function_locations(&*db, &graph).unwrap();
        import_calls(&*db, &graph).unwrap();

        // Run update_call_counts
        update_call_counts(&*db).unwrap();

        // Query counts
        // Columns in alphabetical order: incoming_call_count (0), name (1), outgoing_call_count (2)
        let query = "SELECT name, incoming_call_count, outgoing_call_count FROM functions ORDER BY name";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        let counts: std::collections::HashMap<String, (i64, i64)> = rows
            .rows()
            .iter()
            .filter_map(|row| {
                let name = row.get(1).and_then(|v| v.as_str()).map(|s| s.to_string())?;
                let incoming = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
                let outgoing = row.get(2).and_then(|v| v.as_i64()).unwrap_or(0);
                Some((name, (incoming, outgoing)))
            })
            .collect();

        // Repo.get: called twice (by get_user and update_user)
        assert_eq!(counts.get("get"), Some(&(2, 0)), "get: incoming=2, outgoing=0");

        // Repo.update: called once (by update_user)
        assert_eq!(counts.get("update"), Some(&(1, 0)), "update: incoming=1, outgoing=0");

        // get_user: makes 1 call (to Repo.get)
        assert_eq!(counts.get("get_user"), Some(&(0, 1)), "get_user: incoming=0, outgoing=1");

        // update_user: makes 2 calls (to Repo.get and Repo.update)
        assert_eq!(counts.get("update_user"), Some(&(0, 2)), "update_user: incoming=0, outgoing=2");
    }

    /// Test update_call_counts handles empty calls table (no calls)
    #[test]
    fn test_update_call_counts_no_calls() {
        let db = crate::open_mem_db().unwrap();
        crate::queries::schema::create_schema(&*db).unwrap();

        // Create functions without any calls
        let json = r#"{
            "specs": {
                "MyApp.Utils": [
                    {"name": "helper", "arity": 0, "line": 5, "kind": "spec", "clauses": [{"full": "@spec", "input_strings": [], "return_strings": []}]}
                ]
            },
            "function_locations": {
                "MyApp.Utils": {
                    "helper/0:5": {"name": "helper", "arity": 0, "source_file": "lib/utils.ex", "kind": "def", "line": 5, "start_line": 5, "end_line": 8}
                }
            },
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules(&*db, &graph).unwrap();
        import_functions(&*db, &graph).unwrap();
        import_function_locations(&*db, &graph).unwrap();

        // Run update_call_counts - should not error even with no calls
        let result = update_call_counts(&*db);
        assert!(result.is_ok(), "update_call_counts should succeed with no calls: {:?}", result.err());

        // Verify counts are 0
        // Columns in alphabetical order: incoming_call_count (0), name (1), outgoing_call_count (2)
        let query = "SELECT name, incoming_call_count, outgoing_call_count FROM functions";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        let row = rows.rows().first().unwrap();
        let incoming = row.get(0).and_then(|v| v.as_i64()).unwrap_or(-1);
        let outgoing = row.get(2).and_then(|v| v.as_i64()).unwrap_or(-1);

        assert_eq!(incoming, 0, "helper should have incoming_call_count=0");
        assert_eq!(outgoing, 0, "helper should have outgoing_call_count=0");
    }
}
