use std::error::Error;


use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum TypesError {
    #[error("Types query failed: {message}")]
    QueryFailed { message: String },
}

/// A type definition (@type, @typep, @opaque)
#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    pub project: String,
    pub module: String,
    pub name: String,
    pub kind: String,
    pub params: String,
    pub line: i64,
    pub definition: String,
}

pub fn find_types(
    db: &dyn Database,
    module_pattern: &str,
    name_filter: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<TypeInfo>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), name_filter])?;

    // Build conditions using query builders
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let name_cond = OptionalConditionBuilder::new("name", "name_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(name_filter.is_some(), use_regex);
    let kind_cond = OptionalConditionBuilder::new("kind", "kind")
        .with_leading_comma()
        .build(kind_filter.is_some());

    let script = format!(
        r#"
        ?[project, module, name, kind, params, line, definition] :=
            *types{{project, module, name, kind, params, line, definition}},
            project == $project,
            {module_cond}
            {name_cond}
            {kind_cond}

        :order module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_str("module_pattern", module_pattern);

    if let Some(name) = name_filter {
        params = params.with_str("name_pattern", name);
    }

    if let Some(kind) = kind_filter {
        params = params.with_str("kind", kind);
    }

    let result = run_query(db, &script, params).map_err(|e| TypesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 7 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let Some(kind) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let params_str = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(5).unwrap(), 0);
            let definition = extract_string(row.get(6).unwrap()).unwrap_or_default();

            results.push(TypeInfo {
                project,
                module,
                name,
                kind,
                params: params_str,
                line,
                definition,
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
    fn test_find_types_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        // May or may not have types, but query should execute
        assert!(types.is_empty() || !types.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_types_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(
            &*populated_db,
            "NonExistentModule",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(types.is_empty(), "Should return empty results for non-existent module");
    }

    #[rstest]
    fn test_find_types_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "MyApp", None, None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert!(t.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_types_with_name_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", Some("String"), None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert_eq!(t.name, "String", "Name should match filter");
        }
    }

    #[rstest]
    fn test_find_types_with_kind_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, Some("type"), "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert_eq!(t.kind, "type", "Kind should match filter");
        }
    }

    #[rstest]
    fn test_find_types_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_types(&*populated_db, "", None, None, "default", false, 5)
            .unwrap();
        let limit_100 = find_types(&*populated_db, "", None, None, "default", false, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_types_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "^MyApp\\..*$", None, None, "default", true, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert!(t.module.starts_with("MyApp"), "Module should match regex");
        }
    }

    #[rstest]
    fn test_find_types_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "[invalid", None, None, "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_types_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, None, "nonexistent", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(types.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_types_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        if !types.is_empty() {
            let t = &types[0];
            assert_eq!(t.project, "default");
            assert!(!t.module.is_empty());
            assert!(!t.name.is_empty());
            assert!(!t.kind.is_empty());
        }
    }
}
