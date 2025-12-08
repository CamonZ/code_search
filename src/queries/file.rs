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
    pub start_line: i64,
    pub end_line: i64,
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
        ?[file, module, name, arity, kind, start_line, end_line] :=
            *function_locations{{project, module, name, arity, file, kind, start_line, end_line}},
            project == $project,
            {file_filter}

        :order file, start_line, module, name, arity
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
        if row.len() >= 7 {
            let Some(file) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let Some(kind) = extract_string(&row[4]) else { continue };
            let start_line = extract_i64(&row[5], 0);
            let end_line = extract_i64(&row[6], 0);

            files_map.entry(file).or_default().push(FileFunctionDef {
                module,
                name,
                arity,
                kind,
                start_line,
                end_line,
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
