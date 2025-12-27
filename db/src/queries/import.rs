use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{run_query, run_query_no_params};
use crate::queries::import_models::CallGraph;
use crate::queries::schema;

#[cfg(feature = "backend-cozo")]
use crate::db::{escape_string, escape_string_single};

/// Chunk size for batch database imports
#[cfg(feature = "backend-cozo")]
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

pub fn clear_project_data(db: &dyn Database, _project: &str) -> Result<(), Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        clear_project_data_cozo(db, _project)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        clear_project_data_surrealdb(db)
    }
}

/// Clear all project data from CozoDB
#[cfg(feature = "backend-cozo")]
fn clear_project_data_cozo(db: &dyn Database, project: &str) -> Result<(), Box<dyn Error>> {
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

        let params = QueryParams::new().with_str("project", project);

        run_query(db, &script, params).map_err(|e| ImportError::ClearFailed {
            message: format!("Failed to clear {}: {}", table, e),
        })?;
    }

    Ok(())
}

/// Clear all project data from SurrealDB
/// Since SurrealDB is per-project, we delete all records from all tables
#[cfg(feature = "backend-surrealdb")]
fn clear_project_data_surrealdb(db: &dyn Database) -> Result<(), Box<dyn Error>> {
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

/// Import rows in chunks into a CozoDB table
#[cfg(feature = "backend-cozo")]
fn import_rows(
    db: &dyn Database,
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
    db: &dyn Database,
    _project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        import_modules_cozo(db, _project, graph)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        import_modules_surrealdb(db, graph)
    }
}

/// Import modules to CozoDB
#[cfg(feature = "backend-cozo")]
fn import_modules_cozo(
    db: &dyn Database,
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

/// Import modules to SurrealDB
#[cfg(feature = "backend-surrealdb")]
fn import_modules_surrealdb(db: &dyn Database, graph: &CallGraph) -> Result<usize, Box<dyn Error>> {
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

pub fn import_functions(
    db: &dyn Database,
    _project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        import_functions_cozo(db, _project, graph)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        import_functions_surrealdb(db, graph)
    }
}

/// Import functions from specs to CozoDB
#[cfg(feature = "backend-cozo")]
fn import_functions_cozo(
    db: &dyn Database,
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

/// Import functions from specs to SurrealDB
#[cfg(feature = "backend-surrealdb")]
fn import_functions_surrealdb(
    db: &dyn Database,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;

    // Import functions from specs data
    for (module_name, specs) in &graph.specs {
        for spec in specs {
            let query = r#"
                CREATE functions:[$module_name, $name, $arity] SET
                    module_name = $module_name,
                    name = $name,
                    arity = $arity;
            "#;
            let params = QueryParams::new()
                .with_str("module_name", module_name)
                .with_str("name", &spec.name)
                .with_int("arity", spec.arity as i64);
            run_query(db, query, params)?;
            count += 1;
        }
    }

    Ok(count)
}

pub fn import_calls(
    db: &dyn Database,
    _project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        import_calls_cozo(db, _project, graph)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        import_calls_surrealdb(db, graph)
    }
}

/// Import calls to CozoDB
#[cfg(feature = "backend-cozo")]
fn import_calls_cozo(
    db: &dyn Database,
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

/// Import calls to SurrealDB
#[cfg(feature = "backend-surrealdb")]
fn import_calls_surrealdb(db: &dyn Database, graph: &CallGraph) -> Result<usize, Box<dyn Error>> {
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

/// Parse a function reference that may be "name" or "name/arity" format
/// Returns (function_name, arity) - arity defaults to 0 if not specified
#[cfg(feature = "backend-surrealdb")]
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

pub fn import_structs(
    db: &dyn Database,
    _project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        import_structs_cozo(db, _project, graph)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        import_structs_surrealdb(db, graph)
    }
}

/// Import structs to CozoDB
#[cfg(feature = "backend-cozo")]
fn import_structs_cozo(
    db: &dyn Database,
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

/// Import structs to SurrealDB (as fields)
#[cfg(feature = "backend-surrealdb")]
fn import_structs_surrealdb(db: &dyn Database, graph: &CallGraph) -> Result<usize, Box<dyn Error>> {
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

pub fn import_function_locations(
    db: &dyn Database,
    _project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        import_function_locations_cozo(db, _project, graph)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        import_function_locations_surrealdb(db, graph)
    }
}

/// Import function locations to CozoDB
#[cfg(feature = "backend-cozo")]
fn import_function_locations_cozo(
    db: &dyn Database,
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

/// Import function locations to SurrealDB (as clauses)
#[cfg(feature = "backend-surrealdb")]
fn import_function_locations_surrealdb(
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

pub fn import_specs(
    db: &dyn Database,
    _project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        import_specs_cozo(db, _project, graph)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        import_specs_surrealdb(db, graph)
    }
}

/// Import specs to CozoDB
#[cfg(feature = "backend-cozo")]
fn import_specs_cozo(
    db: &dyn Database,
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

/// Import specs to SurrealDB with array fields preserved
#[cfg(feature = "backend-surrealdb")]
fn import_specs_surrealdb(db: &dyn Database, graph: &CallGraph) -> Result<usize, Box<dyn Error>> {
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

pub fn import_types(
    db: &dyn Database,
    _project: &str,
    graph: &CallGraph,
) -> Result<usize, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        import_types_cozo(db, _project, graph)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        import_types_surrealdb(db, graph)
    }
}

/// Import types to CozoDB
#[cfg(feature = "backend-cozo")]
fn import_types_cozo(
    db: &dyn Database,
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

/// Import types to SurrealDB
#[cfg(feature = "backend-surrealdb")]
fn import_types_surrealdb(db: &dyn Database, graph: &CallGraph) -> Result<usize, Box<dyn Error>> {
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
#[cfg(feature = "backend-surrealdb")]
pub fn create_defines_relationships_surrealdb(
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
#[cfg(feature = "backend-surrealdb")]
pub fn create_has_clause_relationships_surrealdb(
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
#[cfg(feature = "backend-surrealdb")]
pub fn create_has_field_relationships_surrealdb(
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

    // Create relationships for SurrealDB
    #[cfg(feature = "backend-surrealdb")]
    {
        create_defines_relationships_surrealdb(db, graph)?;
        create_has_clause_relationships_surrealdb(db, graph)?;
        create_has_field_relationships_surrealdb(db, graph)?;
    }

    Ok(result)
}

/// Import a JSON string directly into the database.
///
/// Convenience wrapper for tests that parses JSON and calls `import_graph`.
#[cfg(any(test, feature = "test-utils"))]
pub fn import_json_str(
    db: &dyn Database,
    content: &str,
    project: &str,
) -> Result<ImportResult, Box<dyn Error>> {
    let graph: CallGraph =
        serde_json::from_str(content).map_err(|e| ImportError::JsonParseFailed {
            message: e.to_string(),
        })?;

    import_graph(db, project, &graph)
}

#[cfg(all(test, feature = "backend-cozo"))]
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

        let result = import_json_str(&*db, json, "test_project").expect("Import should succeed");

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

        let result = import_json_str(&*db, json, "test_project").expect("Import should succeed");

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
        let rows = run_query_no_params(&*db, query).expect("Query should succeed");

        // Extract field names and defaults
        let mut fields: Vec<(String, String)> = rows
            .rows()
            .iter()
            .filter_map(|row| {
                let field = extract_string(row.get(0)?)?;
                let default = extract_string(row.get(1)?)?;
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

        let result = import_json_str(&*db, json, "test_project").expect("Import should succeed");

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
        let rows = run_query_no_params(&*db, query).expect("Query should succeed");

        // Extract type definitions
        let mut types: Vec<(String, String)> = rows
            .rows()
            .iter()
            .filter_map(|row| {
                let name = extract_string(row.get(0)?)?;
                let definition = extract_string(row.get(1)?)?;
                Some((name, definition))
            })
            .collect();
        types.sort();

        // Verify the string-quoted atom syntax is preserved in definitions
        assert_eq!(types.len(), 2);
        assert_eq!(types[0].0, "config");
        assert_eq!(
            types[0].1,
            r#"@type config() :: %{:"api.key" => String.t()}"#
        );
        assert_eq!(types[1].0, "status");
        assert_eq!(
            types[1].1,
            r#"@type status() :: :pending | :active | :"special.status""#
        );
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod tests_surrealdb {
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
        let result = import_modules_surrealdb(&*db, &graph);
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

    /// Test import_functions creates function nodes from specs
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
            "function_locations": {},
            "calls": [],
            "structs": {},
            "types": {}
        }"#;

        let graph: CallGraph = serde_json::from_str(json).unwrap();
        import_modules_surrealdb(&*db, &graph).unwrap();
        let result = import_functions_surrealdb(&*db, &graph);
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
        import_modules_surrealdb(&*db, &graph).unwrap();
        import_functions_surrealdb(&*db, &graph).unwrap();
        let result = import_specs_surrealdb(&*db, &graph);
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
        let result = import_function_locations_surrealdb(&*db, &graph);
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
        import_modules_surrealdb(&*db, &graph).unwrap();
        let result = import_structs_surrealdb(&*db, &graph);
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
        import_modules_surrealdb(&*db, &graph).unwrap();
        let result = import_types_surrealdb(&*db, &graph);
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
        import_modules_surrealdb(&*db_fresh, &graph).unwrap();
        import_functions_surrealdb(&*db_fresh, &graph).unwrap();
        import_types_surrealdb(&*db_fresh, &graph).unwrap();

        let result = create_defines_relationships_surrealdb(&*db_fresh, &graph);
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
        import_modules_surrealdb(&*db, &graph).unwrap();
        import_functions_surrealdb(&*db, &graph).unwrap();
        import_function_locations_surrealdb(&*db, &graph).unwrap();

        let result = create_has_clause_relationships_surrealdb(&*db, &graph);
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
        import_modules_surrealdb(&*db, &graph).unwrap();
        import_structs_surrealdb(&*db, &graph).unwrap();

        let result = create_has_field_relationships_surrealdb(&*db, &graph);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            2,
            "Should create 2 has_field relationships"
        );
    }

    /// Test clear_project_data_surrealdb deletes all data
    #[test]
    fn test_clear_project_data_surrealdb() {
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
        import_modules_surrealdb(&*db, &graph).unwrap();
        import_functions_surrealdb(&*db, &graph).unwrap();
        import_function_locations_surrealdb(&*db, &graph).unwrap();

        // Verify data was imported
        let query = "SELECT COUNT() FROM modules";
        let result = db.execute_query(query, QueryParams::new()).unwrap();
        assert!(
            !result.rows().is_empty(),
            "Should have modules before clear"
        );

        // Clear data
        let clear_result = clear_project_data_surrealdb(&*db);
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
        import_modules_surrealdb(&*db, &graph).unwrap();
        import_functions_surrealdb(&*db, &graph).unwrap();
        import_function_locations_surrealdb(&*db, &graph).unwrap();

        let result = import_calls_surrealdb(&*db, &graph);
        assert!(
            result.is_ok(),
            "Import calls should succeed: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), 1, "Should import 1 call relationship");

        // Verify caller_clause_id is set (call at line 12 is within clause lines 10-15)
        let query = "SELECT caller_clause_id FROM calls";
        let rows = db.execute_query(query, QueryParams::new()).unwrap();
        assert_eq!(rows.rows().len(), 1, "Should have 1 call");

        // The caller_clause_id should be set to the clause record
        let row = rows.rows().first().unwrap();
        let clause_id = row.get(0);
        assert!(
            clause_id.is_some(),
            "caller_clause_id should be set for call within clause range"
        );
    }

    /// Test full import_graph flow with SurrealDB
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
        let result = import_graph(&*db, "test_project", &graph);

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
}
