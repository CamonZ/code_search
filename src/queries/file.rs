use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};

#[derive(Error, Debug)]
pub enum FileError {
    #[error("File query failed: {message}")]
    QueryFailed { message: String },
}

/// A function defined in a file
#[derive(Debug, Clone, Serialize)]
pub struct FileFunctionDef {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub pattern: String,
    pub guard: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub file: String,
}

/// A file with its function definitions
#[derive(Debug, Clone, Serialize)]
pub struct FileWithFunctions {
    pub file: String,
    pub functions: Vec<FileFunctionDef>,
}

pub fn find_functions_in_file(
    db: &cozo::DbInstance,
    file_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FileWithFunctions>, Box<dyn Error>> {
    // Build file filter
    let file_filter = if use_regex {
        "regex_matches(file, $file_pattern)"
    } else {
        "str_includes(file, $file_pattern)"
    };

    // Query to find all functions in matching files
    let script = format!(
        r#"
        ?[file, module, name, arity, kind, line, start_line, end_line, pattern, guard] :=
            *function_locations{{project, module, name, arity, line, file, kind, start_line, end_line, pattern, guard}},
            project == $project,
            {file_filter}

        :order file, start_line, module, name, arity, line
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    params.insert("file_pattern".to_string(), DataValue::Str(file_pattern.into()));

    let rows = run_query(db, &script, params).map_err(|e| FileError::QueryFailed {
        message: e.to_string(),
    })?;

    // Group results by file
    let mut files_map: std::collections::BTreeMap<String, Vec<FileFunctionDef>> = std::collections::BTreeMap::new();

    for row in rows.rows {
        if row.len() >= 10 {
            let Some(file) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let Some(kind) = extract_string(&row[4]) else { continue };
            let line = extract_i64(&row[5], 0);
            let start_line = extract_i64(&row[6], 0);
            let end_line = extract_i64(&row[7], 0);
            let pattern = extract_string(&row[8]).unwrap_or_default();
            let guard = extract_string(&row[9]).unwrap_or_default();

            files_map.entry(file.clone()).or_default().push(FileFunctionDef {
                module,
                name,
                arity,
                kind,
                line,
                start_line,
                end_line,
                pattern,
                guard,
                file,
            });
        }
    }

    // Convert map to vec
    let results: Vec<FileWithFunctions> = files_map
        .into_iter()
        .map(|(file, functions)| FileWithFunctions { file, functions })
        .collect();

    Ok(results)
}

/// Find all functions in modules matching a pattern
/// Returns a flat vec of functions with location info (for browse-module)
pub fn find_functions_in_module(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FileFunctionDef>, Box<dyn Error>> {
    // Build module filter
    let module_filter = if use_regex {
        "regex_matches(module, $module_pattern)"
    } else {
        "module == $module_pattern"
    };

    // Query to find all functions in matching modules
    let script = format!(
        r#"
        ?[module, name, arity, kind, line, start_line, end_line, file, pattern, guard] :=
            *function_locations{{project, module, name, arity, line, file, kind, start_line, end_line, pattern, guard}},
            project == $project,
            {module_filter}

        :order module, start_line, name, arity, line
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));

    let rows = run_query(db, &script, params).map_err(|e| FileError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();

    for row in rows.rows {
        if row.len() >= 10 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let Some(kind) = extract_string(&row[3]) else { continue };
            let line = extract_i64(&row[4], 0);
            let start_line = extract_i64(&row[5], 0);
            let end_line = extract_i64(&row[6], 0);
            let file = extract_string(&row[7]).unwrap_or_default();
            let pattern = extract_string(&row[8]).unwrap_or_default();
            let guard = extract_string(&row[9]).unwrap_or_default();

            results.push(FileFunctionDef {
                module,
                name,
                arity,
                kind,
                line,
                start_line,
                end_line,
                pattern,
                guard,
                file,
            });
        }
    }

    Ok(results)
}
