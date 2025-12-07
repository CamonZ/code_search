use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::FunctionCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, extract_string_or, open_db, run_query, Params};

#[derive(Error, Debug)]
enum FunctionError {
    #[error("Function query failed: {message}")]
    QueryFailed { message: String },
}

/// A function signature
#[derive(Debug, Clone, Serialize)]
pub struct FunctionSignature {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub args: String,
    pub return_type: String,
}

/// Result of the function command execution
#[derive(Debug, Default, Serialize)]
pub struct FunctionResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub functions: Vec<FunctionSignature>,
}

impl Execute for FunctionCmd {
    type Output = FunctionResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = FunctionResult {
            module_pattern: self.module.clone(),
            function_pattern: self.function.clone(),
            ..Default::default()
        };

        result.functions = find_functions(
            &db,
            &self.module,
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_functions(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionSignature>, Box<dyn Error>> {
    let module_cond = if use_regex {
        "regex_matches(module, $module_pattern)".to_string()
    } else {
        "module == $module_pattern".to_string()
    };

    let function_cond = if use_regex {
        ", regex_matches(name, $function_pattern)".to_string()
    } else {
        ", name == $function_pattern".to_string()
    };

    let arity_cond = if arity.is_some() {
        ", arity == $arity"
    } else {
        ""
    };

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, module, name, arity, args, return_type] :=
            *functions{{project, module, name, arity, args, return_type}},
            {module_cond}
            {function_cond}
            {arity_cond}
            {project_cond}
        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("function_pattern".to_string(), DataValue::Str(function_pattern.into()));
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::from(a));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(&db, &script, params).map_err(|e| FunctionError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let args = extract_string_or(&row[4], "");
            let return_type = extract_string_or(&row[5], "");

            results.push(FunctionSignature {
                project,
                module,
                name,
                arity,
                args,
                return_type,
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
                    "get_user/2": {
                        "arity": 2,
                        "name": "get_user",
                        "file": "lib/my_app/accounts.ex",
                        "column": 3,
                        "kind": "def",
                        "start_line": 20,
                        "end_line": 25
                    }
                },
                "MyApp.Users": {
                    "list_users/0": {
                        "arity": 0,
                        "name": "list_users",
                        "file": "lib/my_app/users.ex",
                        "column": 3,
                        "kind": "def",
                        "start_line": 5,
                        "end_line": 10
                    }
                }
            },
            "calls": [],
            "type_signatures": {
                "MyApp.Accounts": {
                    "get_user/1": {
                        "arity": 1,
                        "name": "get_user",
                        "clauses": [
                            {"return": "User.t() | nil", "args": ["integer()"]}
                        ]
                    },
                    "get_user/2": {
                        "arity": 2,
                        "name": "get_user",
                        "clauses": [
                            {"return": "User.t() | nil", "args": ["integer()", "keyword()"]}
                        ]
                    }
                },
                "MyApp.Users": {
                    "list_users/0": {
                        "arity": 0,
                        "name": "list_users",
                        "clauses": [
                            {"return": "[User.t()]", "args": []}
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
    fn test_function_exact_match(populated_db: NamedTempFile) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Function should succeed");
        assert_eq!(result.functions.len(), 2); // get_user/1 and get_user/2
    }

    #[rstest]
    fn test_function_with_arity(populated_db: NamedTempFile) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Function should succeed");
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].arity, 1);
        assert_eq!(result.functions[0].args, "integer()");
        assert_eq!(result.functions[0].return_type, "User.t() | nil");
    }

    #[rstest]
    fn test_function_regex_match(populated_db: NamedTempFile) {
        let cmd = FunctionCmd {
            module: "MyApp\\..*".to_string(),
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Function should succeed");
        assert_eq!(result.functions.len(), 3); // get_user/1, get_user/2, list_users/0
    }

    #[rstest]
    fn test_function_no_match(populated_db: NamedTempFile) {
        let cmd = FunctionCmd {
            module: "NonExistent".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Function should succeed");
        assert!(result.functions.is_empty());
    }

    #[rstest]
    fn test_function_with_project_filter(populated_db: NamedTempFile) {
        let cmd = FunctionCmd {
            module: "MyApp.Accounts".to_string(),
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Function should succeed");
        assert_eq!(result.functions.len(), 2);
        assert!(result.functions.iter().all(|f| f.project == "test_project"));
    }

    #[rstest]
    fn test_function_with_limit(populated_db: NamedTempFile) {
        let cmd = FunctionCmd {
            module: "MyApp\\..*".to_string(),
            function: ".*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 2,
        };
        let result = cmd.execute(populated_db.path()).expect("Function should succeed");
        assert_eq!(result.functions.len(), 2); // Limited to 2 even though there are 3
    }

    #[rstest]
    fn test_function_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = FunctionCmd {
            module: "MyApp".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
