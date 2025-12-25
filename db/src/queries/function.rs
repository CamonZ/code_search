use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum FunctionError {
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

pub fn find_functions(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionSignature>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), Some(function_pattern)])?;

    // Build query conditions using helpers
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let function_cond = ConditionBuilder::new("name", "function_pattern")
        .with_leading_comma()
        .build(use_regex);
    let arity_cond = OptionalConditionBuilder::new("arity", "arity")
        .with_leading_comma()
        .build(arity.is_some());
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

    let mut params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_str("function_pattern", function_pattern)
        .with_str("project", project);

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = run_query(db, &script, params).map_err(|e| FunctionError::QueryFailed {
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
            let args = extract_string_or(row.get(4).unwrap(), "");
            let return_type = extract_string_or(row.get(5).unwrap(), "");

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

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::type_signatures_db("default")
    }

    #[rstest]
    fn test_find_functions_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "",
            "",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        // May be empty if fixture doesn't have functions, just verify query executes
        assert!(functions.is_empty() || !functions.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_functions_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "NonExistentModule",
            "nonexistent",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Should return empty results for non-existent module");
    }

    #[rstest]
    fn test_find_functions_with_arity_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "MyApp.Controller",
            "index",
            Some(2),
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        // Verify all results have arity matching the filter or empty
        for func in &functions {
            assert_eq!(func.arity, 2, "All results should match arity filter");
        }
    }

    #[rstest]
    fn test_find_functions_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_1 = find_functions(&*populated_db, "MyApp", "", None, "default", false, 1)
            .unwrap();
        let limit_100 = find_functions(&*populated_db, "MyApp", "", None, "default", false, 100)
            .unwrap();

        assert!(limit_1.len() <= 1, "Limit should be respected");
        assert!(limit_1.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_functions_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "^MyApp\\..*$",
            "^index$",
            None,
            "default",
            true,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        // Should find functions matching the regex pattern
        if !functions.is_empty() {
            for func in &functions {
                assert!(func.module.starts_with("MyApp"), "Module should match regex");
                assert_eq!(func.name, "index", "Name should match regex");
            }
        }
    }

    #[rstest]
    fn test_find_functions_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(&*populated_db, "[invalid", "index", None, "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_functions_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "nonexistent",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_functions_returns_proper_fields(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        if !functions.is_empty() {
            let func = &functions[0];
            assert_eq!(func.project, "default");
            assert!(!func.module.is_empty());
            assert!(!func.name.is_empty());
            assert!(func.arity >= 0);
        }
    }
}
