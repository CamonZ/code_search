use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::DependedByCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum DependedByError {
    #[error("Dependency query failed: {message}")]
    QueryFailed { message: String },
}

/// A module that depends on the target, with call count
#[derive(Debug, Clone, Serialize)]
pub struct ModuleDependent {
    pub module: String,
    pub call_count: i64,
}

/// Result of the depended-by command execution
#[derive(Debug, Default, Serialize)]
pub struct DependedByResult {
    pub target_module: String,
    pub dependents: Vec<ModuleDependent>,
}

impl Execute for DependedByCmd {
    type Output = DependedByResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = DependedByResult {
            target_module: self.module.clone(),
            ..Default::default()
        };

        result.dependents = find_dependents(
            &db,
            &self.module,
            self.project.as_deref(),
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_dependents(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: Option<&str>,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<ModuleDependent>, Box<dyn Error>> {
    let module_cond = if use_regex {
        "regex_matches(callee_module, $module_pattern)"
    } else {
        "callee_module == $module_pattern"
    };

    let project_cond = if project.is_some() {
        ", project == $project"
    } else {
        ""
    };

    // Aggregate calls by caller module, excluding self-references
    // In CozoDB, count(callee_module) counts occurrences grouped by caller_module
    let script = format!(
        r#"
        ?[caller_module, count(callee_module)] :=
            *calls{{caller_module, callee_module}},
            {module_cond},
            caller_module != callee_module
            {project_cond}
        :order -count(callee_module), caller_module
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    if let Some(proj) = project {
        params.insert("project".to_string(), DataValue::Str(proj.into()));
    }

    let rows = run_query(db, &script, params).map_err(|e| DependedByError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 2 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let call_count = extract_i64(&row[1], 0);

            results.push(ModuleDependent { module, call_count });
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
                    "caller": {"module": "MyApp.Controller", "function": "show", "file": "lib/controller.ex", "line": 15, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 2, "function": "get", "module": "MyApp.Repo"}
                },
                {
                    "caller": {"module": "MyApp.Service", "function": "fetch", "file": "lib/service.ex", "line": 15, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 2, "function": "get", "module": "MyApp.Repo"}
                },
                {
                    "caller": {"module": "MyApp.Service", "function": "fetch", "file": "lib/service.ex", "line": 16, "column": 5},
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
    fn test_depended_by_single_module(populated_db: NamedTempFile) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            project: None,
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependedBy should succeed");
        assert_eq!(result.dependents.len(), 2); // Controller and Service depend on Repo
        assert!(result.dependents.iter().any(|d| d.module == "MyApp.Controller"));
        assert!(result.dependents.iter().any(|d| d.module == "MyApp.Service"));
    }

    #[rstest]
    fn test_depended_by_counts_calls(populated_db: NamedTempFile) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            project: None,
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependedBy should succeed");
        // Service has 2 calls to Repo, Controller has 1
        let service = result.dependents.iter().find(|d| d.module == "MyApp.Service").unwrap();
        let controller = result.dependents.iter().find(|d| d.module == "MyApp.Controller").unwrap();
        assert_eq!(service.call_count, 2);
        assert_eq!(controller.call_count, 1);
    }

    #[rstest]
    fn test_depended_by_ordered_by_count(populated_db: NamedTempFile) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            project: None,
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependedBy should succeed");
        // Service (2 calls) should come before Controller (1 call)
        assert_eq!(result.dependents[0].module, "MyApp.Service");
        assert_eq!(result.dependents[1].module, "MyApp.Controller");
    }

    #[rstest]
    fn test_depended_by_no_match(populated_db: NamedTempFile) {
        let cmd = DependedByCmd {
            module: "NonExistent".to_string(),
            project: None,
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependedBy should succeed");
        assert!(result.dependents.is_empty());
    }

    #[rstest]
    fn test_depended_by_excludes_self(populated_db: NamedTempFile) {
        let cmd = DependedByCmd {
            module: "MyApp.Repo".to_string(),
            project: None,
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependedBy should succeed");
        assert!(!result.dependents.iter().any(|d| d.module == "MyApp.Repo"));
    }

    #[rstest]
    fn test_depended_by_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = DependedByCmd {
            module: "MyApp".to_string(),
            project: None,
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
