use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::ReverseTraceCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum ReverseTraceError {
    #[error("Reverse trace query failed: {message}")]
    QueryFailed { message: String },
}

/// A single step in the reverse call chain
#[derive(Debug, Clone, Serialize)]
pub struct ReverseTraceStep {
    pub depth: i64,
    pub caller_module: String,
    pub caller_function: String,
    pub callee_module: String,
    pub callee_function: String,
    pub callee_arity: i64,
    pub file: String,
    pub line: i64,
}

/// Result of the reverse-trace command execution
#[derive(Debug, Default, Serialize)]
pub struct ReverseTraceResult {
    pub target_module: String,
    pub target_function: String,
    pub max_depth: u32,
    pub steps: Vec<ReverseTraceStep>,
}

impl Execute for ReverseTraceCmd {
    type Output = ReverseTraceResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = ReverseTraceResult {
            target_module: self.module.clone(),
            target_function: self.function.clone(),
            max_depth: self.depth,
            ..Default::default()
        };

        result.steps = reverse_trace_calls(
            &db,
            &self.module,
            &self.function,
            self.arity,
            self.project.as_deref(),
            self.regex,
            self.depth,
            self.limit,
        )?;

        Ok(result)
    }
}

fn reverse_trace_calls(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: Option<&str>,
    use_regex: bool,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<ReverseTraceStep>, Box<dyn Error>> {
    // Build the starting condition for the recursive query
    // For reverse trace, we match on the callee (target)
    let module_cond = if use_regex {
        "regex_matches(callee_module, $module_pattern)"
    } else {
        "callee_module == $module_pattern"
    };

    let function_cond = if use_regex {
        "regex_matches(callee_function, $function_pattern)"
    } else {
        "callee_function == $function_pattern"
    };

    let arity_cond = if arity.is_some() {
        ", callee_arity == $arity"
    } else {
        ""
    };

    let project_cond = if project.is_some() {
        ", project == $project"
    } else {
        ""
    };

    // Recursive query to trace call chains backwards
    // Base case: calls TO the target function
    // Recursive case: calls TO the callers we've found
    let script = format!(
        r#"
        # Base case: calls to the target function
        trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line}},
            {module_cond},
            {function_cond}
            {arity_cond}
            {project_cond},
            depth = 1

        # Recursive case: calls to the callers we've found
        # Note: prev_caller_function has arity suffix (e.g., "foo/2") but callee_function doesn't (e.g., "foo")
        # So we use starts_with to match prev_caller_function starting with callee_function
        trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            trace[prev_depth, prev_caller_module, prev_caller_function, _, _, _, _, _],
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line}},
            callee_module == prev_caller_module,
            starts_with(prev_caller_function, callee_function),
            prev_depth < {max_depth},
            depth = prev_depth + 1
            {project_cond}

        ?[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line] :=
            trace[depth, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line]

        :order depth, caller_module, caller_function, callee_module, callee_function
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("function_pattern".to_string(), DataValue::Str(function_pattern.into()));
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::from(a));
    }
    if let Some(proj) = project {
        params.insert("project".to_string(), DataValue::Str(proj.into()));
    }

    let rows = run_query(&db, &script, params).map_err(|e| ReverseTraceError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 8 {
            let depth = extract_i64(&row[0], 0);
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_function) = extract_string(&row[2]) else { continue };
            let Some(callee_module) = extract_string(&row[3]) else { continue };
            let Some(callee_function) = extract_string(&row[4]) else { continue };
            let callee_arity = extract_i64(&row[5], 0);
            let Some(file) = extract_string(&row[6]) else { continue };
            let line = extract_i64(&row[7], 0);

            results.push(ReverseTraceStep {
                depth,
                caller_module,
                caller_function,
                callee_module,
                callee_function,
                callee_arity,
                file,
                line,
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
                "MyApp.Controller": {
                    "index/2": {"arity": 2, "name": "index", "file": "lib/controller.ex", "column": 3, "kind": "def", "start_line": 5, "end_line": 10}
                },
                "MyApp.Service": {
                    "fetch/1": {"arity": 1, "name": "fetch", "file": "lib/service.ex", "column": 3, "kind": "def", "start_line": 10, "end_line": 20}
                },
                "MyApp.Repo": {
                    "get/2": {"arity": 2, "name": "get", "file": "lib/repo.ex", "column": 3, "kind": "def", "start_line": 15, "end_line": 25}
                }
            },
            "calls": [
                {
                    "caller": {"module": "MyApp.Controller", "function": "index", "file": "lib/controller.ex", "line": 7, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 1, "function": "fetch", "module": "MyApp.Service"}
                },
                {
                    "caller": {"module": "MyApp.Service", "function": "fetch", "file": "lib/service.ex", "line": 15, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 2, "function": "get", "module": "MyApp.Repo"}
                },
                {
                    "caller": {"module": "MyApp.Repo", "function": "get", "file": "lib/repo.ex", "line": 20, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 1, "function": "query", "module": "Ecto.Query"}
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
    fn test_reverse_trace_single_depth(populated_db: NamedTempFile) {
        let cmd = ReverseTraceCmd {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            project: None,
            regex: false,
            depth: 1,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Reverse trace should succeed");
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].caller_module, "MyApp.Service");
        assert_eq!(result.steps[0].caller_function, "fetch");
    }

    #[rstest]
    fn test_reverse_trace_multiple_depths(populated_db: NamedTempFile) {
        let cmd = ReverseTraceCmd {
            module: "MyApp.Repo".to_string(),
            function: "get".to_string(),
            arity: None,
            project: None,
            regex: false,
            depth: 3,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Reverse trace should succeed");
        // Should find: Service->Repo (depth 1), Controller->Service (depth 2)
        assert_eq!(result.steps.len(), 2);
    }

    #[rstest]
    fn test_reverse_trace_from_leaf(populated_db: NamedTempFile) {
        // Start from Ecto.Query.query which is called by Repo.get
        let cmd = ReverseTraceCmd {
            module: "Ecto.Query".to_string(),
            function: "query".to_string(),
            arity: None,
            project: None,
            regex: false,
            depth: 5,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Reverse trace should succeed");
        // Should find: Repo->Ecto (depth 1), Service->Repo (depth 2), Controller->Service (depth 3)
        assert_eq!(result.steps.len(), 3);
    }

    #[rstest]
    fn test_reverse_trace_no_match(populated_db: NamedTempFile) {
        let cmd = ReverseTraceCmd {
            module: "NonExistent".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: None,
            regex: false,
            depth: 5,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Reverse trace should succeed");
        assert!(result.steps.is_empty());
    }

    #[rstest]
    fn test_reverse_trace_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = ReverseTraceCmd {
            module: "MyApp".to_string(),
            function: "foo".to_string(),
            arity: None,
            project: None,
            regex: false,
            depth: 5,
            limit: 100,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
