use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::DependsOnCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum DependsOnError {
    #[error("Dependency query failed: {message}")]
    QueryFailed { message: String },
}

/// A module dependency with call count
#[derive(Debug, Clone, Serialize)]
pub struct ModuleDependency {
    pub module: String,
    pub call_count: i64,
}

/// Result of the depends-on command execution
#[derive(Debug, Default, Serialize)]
pub struct DependsOnResult {
    pub source_module: String,
    pub dependencies: Vec<ModuleDependency>,
}

impl Execute for DependsOnCmd {
    type Output = DependsOnResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = DependsOnResult {
            source_module: self.module.clone(),
            ..Default::default()
        };

        result.dependencies = find_dependencies(
            &db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_dependencies(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<ModuleDependency>, Box<dyn Error>> {
    let module_cond = if use_regex {
        "regex_matches(caller_module, $module_pattern)"
    } else {
        "caller_module == $module_pattern"
    };

    let project_cond = ", project == $project";

    // Aggregate calls by callee module, excluding self-references
    // In CozoDB, count(caller_module) counts occurrences grouped by callee_module
    let script = format!(
        r#"
        ?[callee_module, count(caller_module)] :=
            *calls{{project, caller_module, callee_module}},
            {module_cond},
            caller_module != callee_module
            {project_cond}
        :order -count(caller_module), callee_module
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| DependsOnError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 2 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let call_count = extract_i64(&row[1], 0);

            results.push(ModuleDependency { module, call_count });
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
                    "caller": {"module": "MyApp.Controller", "function": "index", "file": "lib/controller.ex", "line": 8, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 2, "function": "render", "module": "Phoenix.View"}
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
    fn test_depends_on_single_module(populated_db: NamedTempFile) {
        let cmd = DependsOnCmd {
            module: "MyApp.Controller".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependsOn should succeed");
        assert_eq!(result.dependencies.len(), 2); // Service and Phoenix.View
        assert!(result.dependencies.iter().any(|d| d.module == "MyApp.Service"));
        assert!(result.dependencies.iter().any(|d| d.module == "Phoenix.View"));
    }

    #[rstest]
    fn test_depends_on_counts_calls(populated_db: NamedTempFile) {
        let cmd = DependsOnCmd {
            module: "MyApp.Service".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependsOn should succeed");
        assert_eq!(result.dependencies.len(), 1); // Only Repo
        assert_eq!(result.dependencies[0].module, "MyApp.Repo");
        assert_eq!(result.dependencies[0].call_count, 2); // Two calls to Repo
    }

    #[rstest]
    fn test_depends_on_no_match(populated_db: NamedTempFile) {
        let cmd = DependsOnCmd {
            module: "NonExistent".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependsOn should succeed");
        assert!(result.dependencies.is_empty());
    }

    #[rstest]
    fn test_depends_on_excludes_self(populated_db: NamedTempFile) {
        // Even if a module calls itself, it shouldn't appear in dependencies
        let cmd = DependsOnCmd {
            module: "MyApp.Repo".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("DependsOn should succeed");
        assert!(!result.dependencies.iter().any(|d| d.module == "MyApp.Repo"));
    }

    #[rstest]
    fn test_depends_on_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = DependsOnCmd {
            module: "MyApp".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
