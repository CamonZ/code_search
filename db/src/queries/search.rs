use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder};

#[derive(Error, Debug)]
pub enum SearchError {
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

pub fn search_modules(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<ModuleResult>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern)])?;

    let match_cond = ConditionBuilder::new("name", "pattern").build(use_regex);
    let script = format!(
        r#"
        ?[project, name, source] := *modules{{project, name, source}},
            project = $project,
            {match_cond}
        :limit {limit}
        :order name
        "#,
    );

    let params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 3 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let source = extract_string_or(row.get(2).unwrap(), "");

            results.push(ModuleResult {
                project,
                name,
                source,
            });
        }
    }

    Ok(results)
}

pub fn search_functions(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<FunctionResult>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern)])?;

    let match_cond = ConditionBuilder::new("name", "pattern").build(use_regex);
    let script = format!(
        r#"
        ?[project, module, name, arity, return_type] := *functions{{project, module, name, arity, return_type}},
            project = $project,
            {match_cond}
        :limit {limit}
        :order module, name, arity
        "#,
    );

    let params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 5 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let arity = extract_i64(row.get(3).unwrap(), 0);
            let return_type = extract_string_or(row.get(4).unwrap(), "");

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

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;

    #[test]
    fn test_search_modules_invalid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Invalid regex pattern: unclosed bracket
        let result = search_modules(&*db, "[invalid", "test_project", 10, true);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern"),
            "Error should mention invalid regex: {}",
            msg
        );
        assert!(
            msg.contains("[invalid"),
            "Error should show the pattern: {}",
            msg
        );
    }

    #[test]
    fn test_search_functions_invalid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Invalid regex pattern: invalid repetition
        let result = search_functions(&*db, "*invalid", "test_project", 10, true);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern"),
            "Error should mention invalid regex: {}",
            msg
        );
        assert!(
            msg.contains("*invalid"),
            "Error should show the pattern: {}",
            msg
        );
    }

    #[test]
    fn test_search_modules_valid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Valid regex pattern should not error on validation (may or may not find results)
        let result = search_modules(&*db, "^test.*$", "test_project", 10, true);

        // Should not fail on validation (may return empty results, that's fine)
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_functions_valid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Valid regex pattern should not error on validation
        let result = search_functions(&*db, "^get_.*$", "test_project", 10, true);

        // Should not fail on validation
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_modules_non_regex_mode() {
        let db = crate::test_utils::call_graph_db("default");

        // Even invalid regex should work in non-regex mode (treated as literal string)
        let result = search_modules(&*db, "[invalid", "test_project", 10, false);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_functions_non_regex_mode() {
        let db = crate::test_utils::call_graph_db("default");

        // Even invalid regex should work in non-regex mode
        let result = search_functions(&*db, "*invalid", "test_project", 10, false);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }
}
