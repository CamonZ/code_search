use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::CallsFromCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, extract_string_or, open_db, run_query, Params};

#[derive(Error, Debug)]
enum CallsFromError {
    #[error("Calls query failed: {message}")]
    QueryFailed { message: String },
}

/// A single call edge (outgoing from the caller)
#[derive(Debug, Clone, Serialize)]
pub struct CallEdge {
    pub project: String,
    pub caller_module: String,
    pub caller_function: String,
    pub callee_module: String,
    pub callee_function: String,
    pub callee_arity: i64,
    pub file: String,
    pub line: i64,
    pub call_type: String,
}

/// Result of the calls-from command execution
#[derive(Debug, Default, Serialize)]
pub struct CallsFromResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub calls: Vec<CallEdge>,
}

impl Execute for CallsFromCmd {
    type Output = CallsFromResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = CallsFromResult {
            module_pattern: self.module.clone(),
            function_pattern: self.function.clone().unwrap_or_default(),
            ..Default::default()
        };

        result.calls = find_calls_from(
            &db,
            &self.module,
            self.function.as_deref(),
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_calls_from(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<CallEdge>, Box<dyn Error>> {
    // Build conditions for the caller
    let module_cond = if use_regex {
        "regex_matches(caller_module, $module_pattern)".to_string()
    } else {
        "caller_module == $module_pattern".to_string()
    };

    let function_cond = match function_pattern {
        Some(_) if use_regex => ", regex_matches(caller_function, $function_pattern)".to_string(),
        Some(_) => ", caller_function == $function_pattern".to_string(),
        None => String::new(),
    };

    // Note: arity filtering for calls-from would need to join with function_locations
    // For now, we skip arity filtering on the caller side as calls table doesn't have caller_arity
    let _ = arity; // Acknowledge unused for now

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, call_type] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, call_type}},
            {module_cond}
            {function_cond}
            {project_cond}
        :order callee_module, callee_function, callee_arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    if let Some(fn_pat) = function_pattern {
        params.insert("function_pattern".to_string(), DataValue::Str(fn_pat.into()));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| CallsFromError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 9 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_function) = extract_string(&row[2]) else { continue };
            let Some(callee_module) = extract_string(&row[3]) else { continue };
            let Some(callee_function) = extract_string(&row[4]) else { continue };
            let callee_arity = extract_i64(&row[5], 0);
            let Some(file) = extract_string(&row[6]) else { continue };
            let line = extract_i64(&row[7], 0);
            let call_type = extract_string_or(&row[8], "remote");

            results.push(CallEdge {
                project,
                caller_module,
                caller_function,
                callee_module,
                callee_function,
                callee_arity,
                file,
                line,
                call_type,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::ImportCmd;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_call_graph_json() -> &'static str {
        r#"{
            "structs": {},
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1": {
                        "arity": 1,
                        "name": "get_user",
                        "file": "lib/my_app/accounts.ex",
                        "column": 3,
                        "kind": "def",
                        "start_line": 10,
                        "end_line": 15
                    },
                    "list_users/0": {
                        "arity": 0,
                        "name": "list_users",
                        "file": "lib/my_app/accounts.ex",
                        "column": 3,
                        "kind": "def",
                        "start_line": 20,
                        "end_line": 25
                    }
                }
            },
            "calls": [
                {
                    "caller": {
                        "function": "get_user",
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
                },
                {
                    "caller": {
                        "function": "list_users",
                        "line": 22,
                        "module": "MyApp.Accounts",
                        "file": "lib/my_app/accounts.ex",
                        "column": 5
                    },
                    "type": "remote",
                    "callee": {
                        "arity": 1,
                        "function": "all",
                        "module": "MyApp.Repo"
                    }
                },
                {
                    "caller": {
                        "function": "create_user",
                        "line": 30,
                        "module": "MyApp.Users",
                        "file": "lib/my_app/users.ex",
                        "column": 5
                    },
                    "type": "remote",
                    "callee": {
                        "arity": 2,
                        "function": "insert",
                        "module": "MyApp.Repo"
                    }
                }
            ],
            "type_signatures": {}
        }"#
    }

    fn create_temp_json_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write temp file");
        file
    }

    #[fixture]
    fn populated_db() -> NamedTempFile {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let json_file = create_temp_json_file(sample_call_graph_json());

        let import_cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };
        import_cmd
            .execute(db_file.path())
            .expect("Import should succeed");

        db_file
    }

    #[rstest]
    fn test_calls_from_module(populated_db: NamedTempFile) {
        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("CallsFrom should succeed");
        assert_eq!(result.calls.len(), 2); // get_user calls Repo.get, list_users calls Repo.all
    }

    #[rstest]
    fn test_calls_from_function(populated_db: NamedTempFile) {
        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: Some("get_user".to_string()),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("CallsFrom should succeed");
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].callee_module, "MyApp.Repo");
        assert_eq!(result.calls[0].callee_function, "get");
    }

    #[rstest]
    fn test_calls_from_regex_module(populated_db: NamedTempFile) {
        let cmd = CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("CallsFrom should succeed");
        assert_eq!(result.calls.len(), 3); // All calls from MyApp.Accounts and MyApp.Users
    }

    #[rstest]
    fn test_calls_from_no_match(populated_db: NamedTempFile) {
        let cmd = CallsFromCmd {
            module: "NonExistent".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("CallsFrom should succeed");
        assert!(result.calls.is_empty());
    }

    #[rstest]
    fn test_calls_from_with_project_filter(populated_db: NamedTempFile) {
        let cmd = CallsFromCmd {
            module: "MyApp.Accounts".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("CallsFrom should succeed");
        assert_eq!(result.calls.len(), 2);
        assert!(result.calls.iter().all(|c| c.project == "test_project"));
    }

    #[rstest]
    fn test_calls_from_with_limit(populated_db: NamedTempFile) {
        let cmd = CallsFromCmd {
            module: "MyApp\\..*".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 1,
        };
        let result = cmd.execute(populated_db.path()).expect("CallsFrom should succeed");
        assert_eq!(result.calls.len(), 1);
    }

    #[rstest]
    fn test_calls_from_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = CallsFromCmd {
            module: "MyApp".to_string(),
            function: None,
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
