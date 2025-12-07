use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::FileCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum FileError {
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

/// Result of the file command execution
#[derive(Debug, Default, Serialize)]
pub struct FileResult {
    pub project: String,
    pub file_pattern: String,
    pub files: Vec<FileWithFunctions>,
}

/// A file with its function definitions
#[derive(Debug, Clone, Serialize)]
pub struct FileWithFunctions {
    pub file: String,
    pub functions: Vec<FileFunctionDef>,
}

impl Execute for FileCmd {
    type Output = FileResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = FileResult {
            project: self.project.clone(),
            file_pattern: self.file.clone(),
            ..Default::default()
        };

        result.files = find_functions_in_file(
            &db,
            &self.file,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_functions_in_file(
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

    let rows = run_query(&db, &script, params).map_err(|e| FileError::QueryFailed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    const TEST_JSON: &str = r#"{
        "structs": {},
        "function_locations": {
            "MyApp.Accounts": {
                "get_user/1": {"arity": 1, "name": "get_user", "file": "lib/accounts.ex", "column": 3, "kind": "def", "start_line": 10, "end_line": 20},
                "list_users/0": {"arity": 0, "name": "list_users", "file": "lib/accounts.ex", "column": 3, "kind": "def", "start_line": 25, "end_line": 30},
                "private_helper/1": {"arity": 1, "name": "private_helper", "file": "lib/accounts.ex", "column": 3, "kind": "defp", "start_line": 35, "end_line": 40}
            },
            "MyApp.Service": {
                "process/1": {"arity": 1, "name": "process", "file": "lib/service.ex", "column": 3, "kind": "def", "start_line": 5, "end_line": 15}
            },
            "MyApp.Web": {
                "index/2": {"arity": 2, "name": "index", "file": "lib/web/controller.ex", "column": 3, "kind": "def", "start_line": 10, "end_line": 20}
            }
        },
        "calls": [],
        "type_signatures": {}
    }"#;

    crate::execute_test_fixture! {
        fixture_name: populated_db,
        json: TEST_JSON,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_file_finds_functions,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/accounts.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.files.len(), 1);
            assert_eq!(result.files[0].file, "lib/accounts.ex");
            assert_eq!(result.files[0].functions.len(), 3);
        },
    }

    crate::execute_test! {
        test_name: test_file_substring_match,
        fixture: populated_db,
        cmd: FileCmd {
            file: "accounts".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.files.len(), 1);
            assert_eq!(result.files[0].file, "lib/accounts.ex");
        },
    }

    crate::execute_count_test! {
        test_name: test_file_regex_match,
        fixture: populated_db,
        cmd: FileCmd {
            file: "^lib/[^/]+\\.ex$".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: files,
        expected: 2,
    }

    crate::execute_count_test! {
        test_name: test_file_multiple_files,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        field: files,
        expected: 3,
    }

    crate::execute_test! {
        test_name: test_file_sorted_by_line,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/accounts.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            let funcs = &result.files[0].functions;
            assert_eq!(funcs[0].name, "get_user");
            assert_eq!(funcs[1].name, "list_users");
            assert_eq!(funcs[2].name, "private_helper");
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_file_no_match,
        fixture: populated_db,
        cmd: FileCmd {
            file: "nonexistent.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: files,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_file_with_limit,
        fixture: populated_db,
        cmd: FileCmd {
            file: "lib/".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 2,
        },
        assertions: |result| {
            let total_funcs: usize = result.files.iter().map(|f| f.functions.len()).sum();
            assert!(total_funcs <= 2);
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: FileCmd,
        cmd: FileCmd {
            file: "lib/accounts.ex".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
