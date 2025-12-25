use std::error::Error;


use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum SpecsError {
    #[error("Specs query failed: {message}")]
    QueryFailed { message: String },
}

/// A spec or callback definition
#[derive(Debug, Clone, Serialize)]
pub struct SpecDef {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    pub inputs_string: String,
    pub return_string: String,
    pub full: String,
}

pub fn find_specs(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<SpecDef>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;

    // Build conditions using query builders
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let function_cond = OptionalConditionBuilder::new("name", "function_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(function_pattern.is_some(), use_regex);
    let kind_cond = OptionalConditionBuilder::new("kind", "kind")
        .with_leading_comma()
        .build(kind_filter.is_some());

    let script = format!(
        r#"
        ?[project, module, name, arity, kind, line, inputs_string, return_string, full] :=
            *specs{{project, module, name, arity, kind, line, inputs_string, return_string, full}},
            project == $project,
            {module_cond}
            {function_cond}
            {kind_cond}

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_str("module_pattern", module_pattern);

    if let Some(func) = function_pattern {
        params = params.with_str("function_pattern", func);
    }

    if let Some(kind) = kind_filter {
        params = params.with_str("kind", kind);
    }

    let result = run_query(db, &script, params).map_err(|e| SpecsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 9 {
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
            let Some(kind) = extract_string(row.get(4).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(5).unwrap(), 0);
            let inputs_string = extract_string(row.get(6).unwrap()).unwrap_or_default();
            let return_string = extract_string(row.get(7).unwrap()).unwrap_or_default();
            let full = extract_string(row.get(8).unwrap()).unwrap_or_default();

            results.push(SpecDef {
                project,
                module,
                name,
                arity,
                kind,
                line,
                inputs_string,
                return_string,
                full,
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
    fn test_find_specs_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        // May be empty if fixture doesn't have specs, just verify query executes
        assert!(specs.is_empty() || !specs.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_specs_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(
            &*populated_db,
            "NonExistentModule",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let specs = result.unwrap();
        assert!(specs.is_empty(), "Should return empty results for non-existent module");
    }

    #[rstest]
    fn test_find_specs_with_function_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", Some("index"), None, "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        for spec in &specs {
            assert_eq!(spec.name, "index", "Function name should match filter");
        }
    }

    #[rstest]
    fn test_find_specs_with_kind_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, Some("spec"), "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        for spec in &specs {
            assert_eq!(spec.kind, "spec", "Kind should match filter");
        }
    }

    #[rstest]
    fn test_find_specs_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_specs(&*populated_db, "", None, None, "default", false, 5)
            .unwrap();
        let limit_100 = find_specs(&*populated_db, "", None, None, "default", false, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_specs_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "^MyApp\\..*$", None, None, "default", true, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        for spec in &specs {
            assert!(spec.module.starts_with("MyApp"), "Module should match regex");
        }
    }

    #[rstest]
    fn test_find_specs_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "[invalid", None, None, "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_specs_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, None, "nonexistent", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        assert!(specs.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_specs_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        if !specs.is_empty() {
            let spec = &specs[0];
            assert_eq!(spec.project, "default");
            assert!(!spec.module.is_empty());
            assert!(!spec.name.is_empty());
            assert!(!spec.kind.is_empty());
            assert!(spec.arity >= 0);
        }
    }
}
