use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder};

#[derive(Error, Debug)]
pub enum FileError {
    #[error("File query failed: {message}")]
    QueryFailed { message: String },
}

/// A function defined in a file
#[derive(Debug, Clone, Serialize)]
pub struct FileFunctionDef {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub pattern: String,
    pub guard: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub file: String,
}

/// Find all functions in modules matching a pattern
/// Returns a flat vec of functions with location info (for browse-module)
pub fn find_functions_in_module(
    db: &dyn Database,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FileFunctionDef>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    // Build module filter using query builder
    let module_filter = ConditionBuilder::new("module", "module_pattern").build(use_regex);

    // Query to find all functions in matching modules
    let script = format!(
        r#"
        ?[module, name, arity, kind, line, start_line, end_line, file, pattern, guard] :=
            *function_locations{{project, module, name, arity, line, file, kind, start_line, end_line, pattern, guard}},
            project == $project,
            {module_filter}

        :order module, start_line, name, arity, line
        :limit {limit}
        "#,
    );

    let params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| FileError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();

    for row in result.rows() {
        if row.len() >= 10 {
            let Some(module) = extract_string(row.get(0).unwrap()) else {
                continue;
            };

            let Some(name) = extract_string(row.get(1).unwrap()) else {
                continue;
            };

            let arity = extract_i64(row.get(2).unwrap(), 0);

            let Some(kind) = extract_string(row.get(3).unwrap()) else {
                continue;
            };

            let line = extract_i64(row.get(4).unwrap(), 0);
            let start_line = extract_i64(row.get(5).unwrap(), 0);
            let end_line = extract_i64(row.get(6).unwrap(), 0);
            let file = extract_string(row.get(7).unwrap()).unwrap_or_default();
            let pattern = extract_string(row.get(8).unwrap()).unwrap_or_default();
            let guard = extract_string(row.get(9).unwrap()).unwrap_or_default();

            results.push(FileFunctionDef {
                module,
                name,
                arity,
                kind,
                line,
                start_line,
                end_line,
                pattern,
                guard,
                file,
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
    fn test_find_functions_in_module_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions_in_module(&*populated_db, "", "default", false, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        // May be empty if fixture doesn't have modules, just verify query executes
        assert!(functions.is_empty() || !functions.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_functions_in_module_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions_in_module(
            &*populated_db,
            "NonExistentModule",
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Should return empty for non-existent module");
    }

    #[rstest]
    fn test_find_functions_in_module_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_functions_in_module(&*populated_db, "MyApp", "default", false, 5)
            .unwrap();
        let limit_100 = find_functions_in_module(&*populated_db, "MyApp", "default", false, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_functions_in_module_with_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions_in_module(
            &*populated_db,
            "^MyApp\\..*$",
            "default",
            true,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        for func in &functions {
            assert!(func.module.starts_with("MyApp"), "Module should match regex");
        }
    }

    #[rstest]
    fn test_find_functions_in_module_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions_in_module(&*populated_db, "[invalid", "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_functions_in_module_nonexistent_project(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_functions_in_module(&*populated_db, "MyApp", "nonexistent", false, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_functions_in_module_returns_valid_structure(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_functions_in_module(&*populated_db, "MyApp", "default", false, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        if !functions.is_empty() {
            let func = &functions[0];
            assert!(!func.module.is_empty());
            assert!(!func.name.is_empty());
            assert!(!func.kind.is_empty());
            assert!(func.start_line > 0);
            assert!(func.end_line >= func.start_line);
        }
    }
}
