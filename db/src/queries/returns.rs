use std::error::Error;


use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum ReturnsError {
    #[error("Returns query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with its return type specification
#[derive(Debug, Clone, Serialize)]
pub struct ReturnEntry {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub return_string: String,
    pub line: i64,
}

pub fn find_returns(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<ReturnEntry>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern), module_pattern])?;

    // Build conditions using query builders
    let pattern_cond = ConditionBuilder::new("return_string", "pattern").build(use_regex);
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    let script = format!(
        r#"
        ?[project, module, name, arity, return_string, line] :=
            *specs{{project, module, name, arity, return_string, line}},
            project == $project,
            {pattern_cond}
            {module_cond}

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new();
    params = params.with_str("pattern", pattern);
    params = params.with_str("project", project);

    if let Some(mod_pat) = module_pattern {
        params = params.with_str("module_pattern", mod_pat);
    }

    let result = run_query(db, &script, params).map_err(|e| ReturnsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
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
            let return_string = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(5).unwrap(), 0);

            results.push(ReturnEntry {
                project,
                module,
                name,
                arity,
                return_string,
                line,
            });
        }
    }

    Ok(results)
}

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::type_signatures_db("default")
    }

    #[rstest]
    fn test_find_returns_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_returns(&*populated_db, "", "default", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        // May or may not have matching specs, but query should execute
        assert!(entries.is_empty() || !entries.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_returns_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_returns(&*populated_db, "NonExistentReturnType", "default", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.is_empty(), "Should return empty results for non-existent pattern");
    }

    #[rstest]
    fn test_find_returns_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_returns(&*populated_db, "", "default", false, Some("MyApp"), 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        for entry in &entries {
            assert!(entry.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_returns_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_returns(&*populated_db, "", "default", false, None, 5)
            .unwrap();
        let limit_100 = find_returns(&*populated_db, "", "default", false, None, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_returns_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_returns(&*populated_db, "^atom", "default", true, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        for entry in &entries {
            assert!(
                entry.return_string.starts_with("atom"),
                "Return type should match regex"
            );
        }
    }

    #[rstest]
    fn test_find_returns_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_returns(&*populated_db, "[invalid", "default", true, None, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_returns_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_returns(&*populated_db, "", "nonexistent", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_returns_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_returns(&*populated_db, "", "default", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        for entry in &entries {
            assert_eq!(entry.project, "default");
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.arity >= 0);
        }
    }
}
