use std::error::Error;
use std::path::Path;

use cozo::{DataValue, Num};
use serde::Serialize;
use thiserror::Error;

use super::LocationCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, extract_string_or, open_db, run_query, Params};

#[derive(Error, Debug)]
enum LocationError {
    #[error("Location query failed: {message}")]
    QueryFailed { message: String },
}

/// A function location result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionLocation {
    pub project: String,
    pub file: String,
    pub start_line: i64,
    pub end_line: i64,
    pub module: String,
    pub kind: String,
    pub name: String,
    pub arity: i64,
}

impl FunctionLocation {
    /// Format as file:start_line:end_line
    pub fn format_location(&self) -> String {
        format!("{}:{}:{}", self.file, self.start_line, self.end_line)
    }
}

/// Result of the location command execution
#[derive(Debug, Default, Serialize)]
pub struct LocationResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub locations: Vec<FunctionLocation>,
}

impl Execute for LocationCmd {
    type Output = LocationResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = LocationResult {
            module_pattern: self.module.clone().unwrap_or_default(),
            function_pattern: self.function.clone(),
            ..Default::default()
        };

        result.locations = find_locations(
            &db,
            self.module.as_deref(),
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_locations(
    db: &cozo::DbInstance,
    module_pattern: Option<&str>,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionLocation>, Box<dyn Error>> {
    // Build the query based on whether we're using regex or exact match
    let fn_cond = if use_regex {
        "regex_matches(name, $function_pattern)".to_string()
    } else {
        "name == $function_pattern".to_string()
    };

    let module_cond = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", module == $module_pattern".to_string(),
        None => String::new(),
    };

    let arity_cond = if arity.is_some() {
        ", arity == $arity"
    } else {
        ""
    };

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, file, start_line, end_line, module, kind, name, arity] :=
            *function_locations{{project, module, name, arity, file, kind, start_line, end_line}},
            {fn_cond}
            {module_cond}
            {arity_cond}
            {project_cond}
        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("function_pattern".to_string(), DataValue::Str(function_pattern.into()));
    if let Some(mod_pat) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(mod_pat.into()));
    }
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::Num(Num::Int(a)));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| LocationError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 8 {
            // Order matches query: project, file, start_line, end_line, module, kind, name, arity
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(file) = extract_string(&row[1]) else { continue };
            let start_line = extract_i64(&row[2], 0);
            let end_line = extract_i64(&row[3], 0);
            let Some(module) = extract_string(&row[4]) else { continue };
            let kind = extract_string_or(&row[5], "");
            let Some(name) = extract_string(&row[6]) else { continue };
            let arity = extract_i64(&row[7], 0);

            results.push(FunctionLocation {
                project,
                file,
                start_line,
                end_line,
                module,
                kind,
                name,
                arity,
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
                },
                "MyApp.Users": {
                    "create_user/1": {
                        "arity": 1,
                        "name": "create_user",
                        "file": "lib/my_app/users.ex",
                        "column": 3,
                        "kind": "def",
                        "start_line": 5,
                        "end_line": 12
                    }
                }
            },
            "calls": [],
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
    fn test_location_exact_match(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 1);
        assert_eq!(result.locations[0].file, "lib/my_app/accounts.ex");
        assert_eq!(result.locations[0].start_line, 10);
        assert_eq!(result.locations[0].end_line, 15);
    }

    #[rstest]
    fn test_location_without_module(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 1);
        assert_eq!(result.locations[0].module, "MyApp.Accounts");
    }

    #[rstest]
    fn test_location_without_module_multiple_matches(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: None,
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 3); // get_user, list_users, create_user across modules
    }

    #[rstest]
    fn test_location_without_arity(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 1);
    }

    #[rstest]
    fn test_location_with_regex(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: Some("MyApp\\..*".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 3); // get_user, list_users, create_user
    }

    #[rstest]
    fn test_location_no_match(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: Some("NonExistent".to_string()),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert!(result.locations.is_empty());
    }

    #[rstest]
    fn test_location_with_project_filter(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 1);
        assert_eq!(result.locations[0].project, "test_project");
    }

    #[rstest]
    fn test_location_format(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: "get_user".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations[0].format_location(), "lib/my_app/accounts.ex:10:15");
    }

    #[rstest]
    fn test_location_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = LocationCmd {
            module: Some("MyApp".to_string()),
            function: "foo".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        // This will fail because tables don't exist
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }

    #[rstest]
    fn test_location_arity_filter_without_module(populated_db: NamedTempFile) {
        // Find all functions with arity 1 across all modules
        let cmd = LocationCmd {
            module: None,
            function: ".*".to_string(),
            arity: Some(1),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 2); // get_user/1 and create_user/1
        assert!(result.locations.iter().all(|l| l.arity == 1));
    }

    #[rstest]
    fn test_location_project_filter_without_module(populated_db: NamedTempFile) {
        // Find function by name in specific project, without specifying module
        let cmd = LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 1);
        assert_eq!(result.locations[0].project, "test_project");
    }

    #[rstest]
    fn test_location_nonexistent_project(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: None,
            function: "get_user".to_string(),
            arity: None,
            project: "nonexistent_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert!(result.locations.is_empty());
    }

    #[rstest]
    fn test_location_function_regex_with_exact_module(populated_db: NamedTempFile) {
        // Exact module match with regex function pattern
        let cmd = LocationCmd {
            module: Some("MyApp.Accounts".to_string()),
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 2); // get_user and list_users in MyApp.Accounts
    }

    #[rstest]
    fn test_location_arity_zero(populated_db: NamedTempFile) {
        let cmd = LocationCmd {
            module: None,
            function: "list_users".to_string(),
            arity: Some(0),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 1);
        assert_eq!(result.locations[0].arity, 0);
    }

    #[rstest]
    fn test_location_with_limit(populated_db: NamedTempFile) {
        // Search for all user-related functions but limit to 1 result
        let cmd = LocationCmd {
            module: None,
            function: ".*user.*".to_string(),
            arity: None,
            project: "test_project".to_string(),
            regex: true,
            limit: 1,
        };
        let result = cmd.execute(populated_db.path()).expect("Location should succeed");
        assert_eq!(result.locations.len(), 1);
    }
}
