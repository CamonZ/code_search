use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::{SearchCmd, SearchKind};
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, extract_string_or, open_db, run_query, Params};

#[derive(Error, Debug)]
enum SearchError {
    #[error("Search failed: {message}")]
    QueryFailed { message: String },
}

/// A module search result
#[derive(Debug, Clone, Serialize)]
pub struct ModuleResult {
    pub project: String,
    pub name: String,
    pub source: String,
}

/// A function search result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionResult {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub return_type: String,
}

/// Result of the search command execution
#[derive(Debug, Default, Serialize)]
pub struct SearchResult {
    pub pattern: String,
    pub kind: String,
    pub modules: Vec<ModuleResult>,
    pub functions: Vec<FunctionResult>,
}

impl Execute for SearchCmd {
    type Output = SearchResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = SearchResult {
            pattern: self.pattern.clone(),
            kind: match self.kind {
                SearchKind::Modules => "modules".to_string(),
                SearchKind::Functions => "functions".to_string(),
            },
            ..Default::default()
        };

        match self.kind {
            SearchKind::Modules => {
                result.modules = search_modules(&db, &self.pattern, &self.project, self.limit, self.regex)?;
            }
            SearchKind::Functions => {
                result.functions = search_functions(&db, &self.pattern, &self.project, self.limit, self.regex)?;
            }
        }

        Ok(result)
    }
}

fn search_modules(
    db: &cozo::DbInstance,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<ModuleResult>, Box<dyn Error>> {
    let match_fn = if use_regex { "regex_matches" } else { "str_includes" };
    let script = format!(
        r#"
        ?[project, name, source] := *modules{{project, name, source}},
            project = $project,
            {match_fn}(name, $pattern)
        :limit {limit}
        :order name
        "#,
    );

    let mut params = Params::new();
    params.insert("pattern".to_string(), DataValue::Str(pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 3 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let source = extract_string_or(&row[2], "unknown");
            results.push(ModuleResult { project, name, source });
        }
    }

    Ok(results)
}

fn search_functions(
    db: &cozo::DbInstance,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<FunctionResult>, Box<dyn Error>> {
    let match_fn = if use_regex { "regex_matches" } else { "str_includes" };
    let script = format!(
        r#"
        ?[project, module, name, arity, return_type] := *functions{{project, module, name, arity, return_type}},
            project = $project,
            {match_fn}(name, $pattern)
        :limit {limit}
        :order module, name, arity
        "#,
    );

    let mut params = Params::new();
    params.insert("pattern".to_string(), DataValue::Str(pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 5 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let return_type = extract_string_or(&row[4], "");
            results.push(FunctionResult {
                project,
                module,
                name,
                arity,
                return_type,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    const TEST_JSON: &str = r#"{
        "structs": {},
        "function_locations": {},
        "calls": [],
        "type_signatures": {
            "MyApp.Accounts": {
                "get_user/1": {"arity": 1, "name": "get_user", "clauses": [{"return": "User.t()", "args": ["integer()"]}]},
                "list_users/0": {"arity": 0, "name": "list_users", "clauses": [{"return": "list(User.t())", "args": []}]}
            },
            "MyApp.Users": {
                "create_user/1": {"arity": 1, "name": "create_user", "clauses": [{"return": "User.t()", "args": ["map()"]}]}
            }
        }
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
        test_name: test_search_modules_all,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "MyApp".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        assertions: |result| {
            assert_eq!(result.kind, "modules");
            assert_eq!(result.modules.len(), 2);
        },
    }

    crate::execute_test! {
        test_name: test_search_functions_all,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "user".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        assertions: |result| {
            assert_eq!(result.kind, "functions");
            assert_eq!(result.functions.len(), 3);
        },
    }

    crate::execute_test! {
        test_name: test_search_functions_specific,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "get_".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 1);
            assert_eq!(result.functions[0].name, "get_user");
            assert_eq!(result.functions[0].arity, 1);
        },
    }

    crate::execute_test! {
        test_name: test_search_functions_with_regex,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "^get_".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: true,
        },
        assertions: |result| {
            assert_eq!(result.functions.len(), 1);
            assert_eq!(result.functions[0].name, "get_user");
        },
    }

    crate::execute_count_test! {
        test_name: test_search_modules_with_regex,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "\\.(Accounts|Users)$".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: true,
        },
        field: modules,
        expected: 2,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_search_modules_no_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "NonExistent".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        empty_field: modules,
    }

    crate::execute_no_match_test! {
        test_name: test_search_regex_no_match,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "^xyz".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 100,
            regex: true,
        },
        empty_field: functions,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
        test_name: test_search_modules_with_project_filter,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "App".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
        collection: modules,
        condition: |m| m.project == "test_project",
    }

    crate::execute_limit_test! {
        test_name: test_search_with_limit,
        fixture: populated_db,
        cmd: SearchCmd {
            pattern: "user".to_string(),
            kind: SearchKind::Functions,
            project: "test_project".to_string(),
            limit: 1,
            regex: false,
        },
        collection: functions,
        limit: 1,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: SearchCmd,
        cmd: SearchCmd {
            pattern: "test".to_string(),
            kind: SearchKind::Modules,
            project: "test_project".to_string(),
            limit: 100,
            regex: false,
        },
    }
}
