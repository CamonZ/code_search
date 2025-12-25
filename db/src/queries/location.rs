use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum LocationError {
    #[error("Location query failed: {message}")]
    QueryFailed { message: String },
}

/// A function location result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionLocation {
    pub project: String,
    pub file: String,
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub module: String,
    pub kind: String,
    pub name: String,
    pub arity: i64,
    pub pattern: String,
    pub guard: String,
}

pub fn find_locations(
    db: &dyn Database,
    module_pattern: Option<&str>,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionLocation>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern, Some(function_pattern)])?;

    // Build conditions using query builders
    let fn_cond = ConditionBuilder::new("name", "function_pattern").build(use_regex);
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    let arity_cond = if arity.is_some() {
        ", arity == $arity"
    } else {
        ""
    };

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, file, line, start_line, end_line, module, kind, name, arity, pattern, guard] :=
            *function_locations{{project, module, name, arity, line, file, kind, start_line, end_line, pattern, guard}},
            {fn_cond}
            {module_cond}
            {arity_cond}
            {project_cond}
        :order module, name, arity, line
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("function_pattern", function_pattern)
        .with_str("project", project);

    if let Some(mod_pat) = module_pattern {
        params = params.with_str("module_pattern", mod_pat);
    }

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = run_query(db, &script, params).map_err(|e| LocationError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 11 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(file) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(2).unwrap(), 0);
            let start_line = extract_i64(row.get(3).unwrap(), 0);
            let end_line = extract_i64(row.get(4).unwrap(), 0);
            let Some(module) = extract_string(row.get(5).unwrap()) else {
                continue;
            };
            let kind = extract_string_or(row.get(6).unwrap(), "");
            let Some(name) = extract_string(row.get(7).unwrap()) else {
                continue;
            };
            let arity = extract_i64(row.get(8).unwrap(), 0);
            let pattern = extract_string_or(row.get(9).unwrap(), "");
            let guard = extract_string_or(row.get(10).unwrap(), "");

            results.push(FunctionLocation {
                project,
                file,
                line,
                start_line,
                end_line,
                module,
                kind,
                name,
                arity,
                pattern,
                guard,
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
        crate::test_utils::call_graph_db("default")
    }

    #[rstest]
    fn test_find_locations_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(&*populated_db, None, "index", None, "default", false, 100);
        assert!(result.is_ok());
        let locations = result.unwrap();
        assert!(!locations.is_empty(), "Should find function locations");
    }

    #[rstest]
    fn test_find_locations_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(
            &*populated_db,
            None,
            "nonexistent_function",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let locations = result.unwrap();
        assert!(locations.is_empty(), "Should return empty results for non-existent function");
    }

    #[rstest]
    fn test_find_locations_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(
            &*populated_db,
            Some("MyApp.Controller"),
            "index",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let locations = result.unwrap();
        // All results should have the specified module
        for loc in &locations {
            assert_eq!(loc.module, "MyApp.Controller", "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_locations_with_arity_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(&*populated_db, None, "index", Some(2), "default", false, 100);
        assert!(result.is_ok());
        let locations = result.unwrap();
        // All results should match arity
        for loc in &locations {
            assert_eq!(loc.arity, 2, "Arity should match filter");
        }
    }

    #[rstest]
    fn test_find_locations_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_1 = find_locations(&*populated_db, None, "", None, "default", false, 1)
            .unwrap();
        let limit_100 = find_locations(&*populated_db, None, "", None, "default", false, 100)
            .unwrap();

        assert!(limit_1.len() <= 1, "Limit should be respected");
        assert!(limit_1.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_locations_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(&*populated_db, None, "^index$", None, "default", true, 100);
        assert!(result.is_ok());
        let locations = result.unwrap();
        // All results should match the regex pattern
        if !locations.is_empty() {
            for loc in &locations {
                assert_eq!(loc.name, "index", "Name should match regex pattern");
            }
        }
    }

    #[rstest]
    fn test_find_locations_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(&*populated_db, None, "[invalid", None, "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_locations_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(
            &*populated_db,
            None,
            "index",
            None,
            "nonexistent",
            false,
            100,
        );
        assert!(result.is_ok());
        let locations = result.unwrap();
        assert!(locations.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_locations_returns_proper_fields(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_locations(&*populated_db, None, "index", None, "default", false, 100);
        assert!(result.is_ok());
        let locations = result.unwrap();
        if !locations.is_empty() {
            let loc = &locations[0];
            assert_eq!(loc.project, "default");
            assert!(!loc.file.is_empty());
            assert!(loc.line > 0);
            assert!(loc.start_line > 0);
            assert!(loc.end_line >= loc.start_line);
            assert!(!loc.module.is_empty());
            assert!(!loc.name.is_empty());
        }
    }
}
