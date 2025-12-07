use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::UnusedCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum UnusedError {
    #[error("Unused query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that is never called
#[derive(Debug, Clone, Serialize)]
pub struct UnusedFunction {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub file: String,
    pub line: i64,
}

/// Result of the unused command execution
#[derive(Debug, Default, Serialize)]
pub struct UnusedResult {
    pub project: String,
    pub module_filter: Option<String>,
    pub private_only: bool,
    pub public_only: bool,
    pub exclude_generated: bool,
    pub functions: Vec<UnusedFunction>,
}

impl Execute for UnusedCmd {
    type Output = UnusedResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = UnusedResult {
            project: self.project.clone(),
            module_filter: self.module.clone(),
            private_only: self.private_only,
            public_only: self.public_only,
            exclude_generated: self.exclude_generated,
            ..Default::default()
        };

        result.functions = find_unused_functions(
            &db,
            self.module.as_deref(),
            &self.project,
            self.regex,
            self.private_only,
            self.public_only,
            self.exclude_generated,
            self.limit,
        )?;

        Ok(result)
    }
}

/// Generated function name patterns to exclude (Elixir compiler-generated)
const GENERATED_PATTERNS: &[&str] = &[
    "__struct__",
    "__using__",
    "__before_compile__",
    "__after_compile__",
    "__on_definition__",
    "__impl__",
    "__info__",
    "__protocol__",
    "__deriving__",
    "__changeset__",
    "__schema__",
    "__meta__",
];

fn find_unused_functions(
    db: &cozo::DbInstance,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    private_only: bool,
    public_only: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<UnusedFunction>, Box<dyn Error>> {
    // Build optional module filter
    let module_filter = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", str_includes(module, $module_pattern)".to_string(),
        None => String::new(),
    };

    // Build kind filter for private_only/public_only
    let kind_filter = if private_only {
        ", (kind == \"defp\" or kind == \"defmacrop\")".to_string()
    } else if public_only {
        ", (kind == \"def\" or kind == \"defmacro\")".to_string()
    } else {
        String::new()
    };

    // Find functions that exist in function_locations but are never called
    // We use function_locations as the source of "defined functions" and check
    // if they appear as a callee in the calls table
    let script = format!(
        r#"
        # All defined functions
        defined[module, name, arity, kind, file, start_line] :=
            *function_locations{{project, module, name, arity, kind, file, start_line}},
            project == $project
            {module_filter}
            {kind_filter}

        # All functions that are called (as callees)
        called[module, name, arity] :=
            *calls{{project, callee_module, callee_function, callee_arity}},
            project == $project,
            module = callee_module,
            name = callee_function,
            arity = callee_arity

        # Functions that are defined but never called
        ?[module, name, arity, kind, file, line] :=
            defined[module, name, arity, kind, file, line],
            not called[module, name, arity]

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    if let Some(pattern) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(pattern.into()));
    }

    let rows = run_query(&db, &script, params).map_err(|e| UnusedError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let Some(kind) = extract_string(&row[3]) else { continue };
            let Some(file) = extract_string(&row[4]) else { continue };
            let line = extract_i64(&row[5], 0);

            // Filter out generated functions if requested
            if exclude_generated && GENERATED_PATTERNS.iter().any(|p| name.starts_with(p)) {
                continue;
            }

            results.push(UnusedFunction {
                module,
                name,
                arity,
                kind,
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
                "MyApp.Accounts": {
                    "get_user/1": {"arity": 1, "name": "get_user", "file": "lib/accounts.ex", "column": 3, "kind": "def", "start_line": 10, "end_line": 20},
                    "list_users/0": {"arity": 0, "name": "list_users", "file": "lib/accounts.ex", "column": 3, "kind": "def", "start_line": 25, "end_line": 30},
                    "unused_private/0": {"arity": 0, "name": "unused_private", "file": "lib/accounts.ex", "column": 3, "kind": "defp", "start_line": 35, "end_line": 40}
                },
                "MyApp.Service": {
                    "process/1": {"arity": 1, "name": "process", "file": "lib/service.ex", "column": 3, "kind": "def", "start_line": 5, "end_line": 15}
                }
            },
            "calls": [
                {
                    "caller": {"module": "MyApp.Web", "function": "index", "file": "lib/web.ex", "line": 10, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 1, "function": "get_user", "module": "MyApp.Accounts"}
                },
                {
                    "caller": {"module": "MyApp.Web", "function": "list", "file": "lib/web.ex", "line": 20, "column": 5},
                    "type": "remote",
                    "callee": {"arity": 0, "function": "list_users", "module": "MyApp.Accounts"}
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
    fn test_unused_finds_uncalled_functions(populated_db: NamedTempFile) {
        let cmd = UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Unused should succeed");
        // unused_private and process are never called
        assert_eq!(result.functions.len(), 2);
        let names: Vec<&str> = result.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"unused_private"));
        assert!(names.contains(&"process"));
    }

    #[rstest]
    fn test_unused_with_module_filter(populated_db: NamedTempFile) {
        let cmd = UnusedCmd {
            module: Some("Accounts".to_string()),
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Unused should succeed");
        // Only unused_private from MyApp.Accounts
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "unused_private");
    }

    #[rstest]
    fn test_unused_with_regex_filter(populated_db: NamedTempFile) {
        let cmd = UnusedCmd {
            module: Some("^MyApp\\.Service$".to_string()),
            project: "test_project".to_string(),
            regex: true,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Unused should succeed");
        // Only process from MyApp.Service
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "process");
    }

    #[rstest]
    fn test_unused_with_limit(populated_db: NamedTempFile) {
        let cmd = UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 1,
        };
        let result = cmd.execute(populated_db.path()).expect("Unused should succeed");
        assert_eq!(result.functions.len(), 1);
    }

    #[rstest]
    fn test_unused_no_match(populated_db: NamedTempFile) {
        let cmd = UnusedCmd {
            module: Some("NonExistent".to_string()),
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Unused should succeed");
        assert!(result.functions.is_empty());
    }

    #[rstest]
    fn test_unused_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }

    #[rstest]
    fn test_unused_private_only(populated_db: NamedTempFile) {
        let cmd = UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: true,
            public_only: false,
            exclude_generated: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Unused should succeed");
        // Only unused_private is defp, process is def
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "unused_private");
        assert_eq!(result.functions[0].kind, "defp");
    }

    #[rstest]
    fn test_unused_public_only(populated_db: NamedTempFile) {
        let cmd = UnusedCmd {
            module: None,
            project: "test_project".to_string(),
            regex: false,
            private_only: false,
            public_only: true,
            exclude_generated: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Unused should succeed");
        // Only process is def, unused_private is defp
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "process");
        assert_eq!(result.functions[0].kind, "def");
    }
}
