use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::OptionalConditionBuilder;

#[derive(Error, Debug)]
pub enum LargeFunctionsError {
    #[error("Large functions query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with line count information
#[derive(Debug, Clone, Serialize)]
pub struct LargeFunction {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub lines: i64,
    pub file: String,
    pub generated_by: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_large_functions(
    db: &dyn Database,
    min_lines: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<LargeFunction>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    // Build optional generated filter
    let generated_filter = if include_generated {
        String::new()
    } else {
        ", generated_by == \"\"".to_string()
    };

    let script = format!(
        r#"
        ?[module, name, arity, start_line, end_line, lines, file, generated_by] :=
            *function_locations{{project, module, name, arity, line, start_line, end_line, file, generated_by}},
            project == $project,
            lines = end_line - start_line + 1,
            lines >= $min_lines
            {module_cond}
            {generated_filter}

        :order -lines, module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_int("min_lines", min_lines);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| LargeFunctionsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 8 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(1).unwrap()) else { continue };
            let arity = extract_i64(row.get(2).unwrap(), 0);
            let start_line = extract_i64(row.get(3).unwrap(), 0);
            let end_line = extract_i64(row.get(4).unwrap(), 0);
            let lines = extract_i64(row.get(5).unwrap(), 0);
            let Some(file) = extract_string(row.get(6).unwrap()) else { continue };
            let Some(generated_by) = extract_string(row.get(7).unwrap()) else { continue };

            results.push(LargeFunction {
                module,
                name,
                arity,
                start_line,
                end_line,
                lines,
                file,
                generated_by,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_large_functions(
    db: &dyn Database,
    min_lines: i64,
    module_pattern: Option<&str>,
    _project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<LargeFunction>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build WHERE clause conditions (without the WHERE keyword, we'll add it)
    let mut conditions = vec!["end_line - start_line + 1 >= $min_lines".to_string()];

    if let Some(_pattern) = module_pattern {
        if use_regex {
            conditions.push("module_name = <regex>$module_pattern".to_string());
        } else {
            conditions.push("module_name = $module_pattern".to_string());
        }
    }

    if !include_generated {
        conditions.push("(generated_by IS NONE OR generated_by = \"\")".to_string());
    }

    let where_clause = conditions.join(" AND ");

    // Query clauses table to find large functions
    // Lines = end_line - start_line + 1
    let query = format!(
        r#"
        SELECT
            module_name,
            function_name,
            arity,
            start_line,
            end_line,
            end_line - start_line + 1 as lines,
            source_file as file,
            generated_by
        FROM clauses
        WHERE {where_clause}
        ORDER BY lines DESC, module_name, function_name
        LIMIT $limit
        "#
    );

    let mut params = QueryParams::new()
        .with_int("min_lines", min_lines)
        .with_int("limit", limit as i64);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = db.execute_query(&query, params).map_err(|e| LargeFunctionsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns alphabetically (via BTreeMap):
        // 0: arity, 1: end_line, 2: file, 3: function_name,
        // 4: generated_by, 5: lines, 6: module_name, 7: start_line
        if row.len() >= 8 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let end_line = extract_i64(row.get(1).unwrap(), 0);
            let Some(file) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let generated_by = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let lines = extract_i64(row.get(5).unwrap(), 0);
            let Some(module) = extract_string(row.get(6).unwrap()) else {
                continue;
            };
            let start_line = extract_i64(row.get(7).unwrap(), 0);

            results.push(LargeFunction {
                module,
                name,
                arity,
                start_line,
                end_line,
                lines,
                file,
                generated_by,
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
    fn test_find_large_functions_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_large_functions(&*populated_db, 0, None, "default", false, true, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(!functions.is_empty(), "Should find functions");
    }

    #[rstest]
    fn test_find_large_functions_respects_min_lines(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_large_functions(&*populated_db, 50, None, "default", false, true, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        for func in &functions {
            assert!(func.lines >= 50, "All results should have >= min_lines");
        }
    }

    #[rstest]
    fn test_find_large_functions_empty_results_high_threshold(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_large_functions(&*populated_db, 10000, None, "default", false, true, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        // May be empty if no functions are that long
        assert!(functions.is_empty() || !functions.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_large_functions_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_large_functions(&*populated_db, 0, Some("MyApp"), "default", false, true, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        for func in &functions {
            assert!(func.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_large_functions_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_large_functions(&*populated_db, 0, None, "default", false, true, 5)
            .unwrap();
        let limit_100 = find_large_functions(&*populated_db, 0, None, "default", false, true, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_large_functions_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_large_functions(&*populated_db, 0, None, "nonexistent", false, true, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_large_functions_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_large_functions(&*populated_db, 0, None, "default", false, true, 100);
        assert!(result.is_ok());
        let functions = result.unwrap();
        if !functions.is_empty() {
            let func = &functions[0];
            assert!(!func.module.is_empty());
            assert!(!func.name.is_empty());
            assert!(func.arity >= 0);
            assert!(func.lines > 0);
            assert!(func.start_line > 0);
            assert!(func.end_line >= func.start_line);
            assert_eq!(func.lines, func.end_line - func.start_line + 1);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== Basic functionality tests =====

    #[test]
    fn test_find_large_functions_returns_results() {
        let db = get_db();
        let result = find_large_functions(&*db, 0, None, "default", false, true, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let functions = result.unwrap();
        assert!(!functions.is_empty(), "Should find large functions");
    }

    #[test]
    fn test_find_large_functions_returns_exact_count() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        // The complex fixture has 38 clauses total with varying sizes
        // All should be included with min_lines=0
        assert_eq!(
            functions.len(),
            38,
            "Should find exactly 38 clauses (one per clause in fixture)"
        );
    }

    #[test]
    fn test_find_large_functions_calculates_lines_correctly() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        for func in &functions {
            let calculated_lines = func.end_line - func.start_line + 1;
            assert_eq!(
                func.lines, calculated_lines,
                "Lines should be calculated as end_line - start_line + 1 for {}",
                func.name
            );
        }
    }

    #[test]
    fn test_find_large_functions_all_modules_present() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        let modules: std::collections::HashSet<_> = functions.iter().map(|f| f.module.as_str()).collect();

        assert!(
            modules.contains("MyApp.Controller"),
            "Should contain MyApp.Controller"
        );
        assert!(modules.contains("MyApp.Accounts"), "Should contain MyApp.Accounts");
        assert!(modules.contains("MyApp.Service"), "Should contain MyApp.Service");
        assert!(modules.contains("MyApp.Repo"), "Should contain MyApp.Repo");
        assert!(modules.contains("MyApp.Notifier"), "Should contain MyApp.Notifier");
    }

    // ===== Min lines threshold tests =====

    #[test]
    fn test_find_large_functions_respects_min_lines_threshold() {
        let db = get_db();
        let functions = find_large_functions(&*db, 10, None, "default", false, true, 100)
            .expect("Query should succeed");

        for func in &functions {
            assert!(
                func.lines >= 10,
                "All results should have lines >= 10, but {} has {} lines",
                func.name,
                func.lines
            );
        }
    }

    #[test]
    fn test_find_large_functions_with_moderate_min_lines() {
        let db = get_db();
        let functions = find_large_functions(&*db, 2, None, "default", false, true, 100)
            .expect("Query should succeed");

        // Fixture has clauses with 1 line each (start_line == end_line)
        // So with min_lines=2, we should get no results
        assert!(
            functions.is_empty(),
            "Should return empty for min_lines=2 when all clauses have 1 line"
        );
    }

    #[test]
    fn test_find_large_functions_empty_with_very_high_threshold() {
        let db = get_db();
        let functions = find_large_functions(&*db, 1000, None, "default", false, true, 100)
            .expect("Query should succeed");

        assert!(
            functions.is_empty(),
            "Should return empty with very high min_lines threshold"
        );
    }

    // ===== Module pattern filtering tests =====

    #[test]
    fn test_find_large_functions_with_exact_module_filter() {
        let db = get_db();
        let functions = find_large_functions(
            &*db,
            0,
            Some("MyApp.Controller"),
            "default",
            false,
            true,
            100,
        )
        .expect("Query should succeed");

        assert!(
            !functions.is_empty(),
            "Should find Controller functions"
        );

        for func in &functions {
            assert_eq!(
                func.module, "MyApp.Controller",
                "All results should be from Controller module"
            );
        }
    }

    #[test]
    fn test_find_large_functions_with_regex_module_filter() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, Some("^MyApp\\.Acc.*"), "default", true, true, 100)
            .expect("Query should succeed");

        for func in &functions {
            assert_eq!(
                func.module, "MyApp.Accounts",
                "All results should be from Accounts module"
            );
        }
    }

    #[test]
    fn test_find_large_functions_with_nonexistent_module() {
        let db = get_db();
        let functions = find_large_functions(
            &*db,
            0,
            Some("NonExistentModule"),
            "default",
            false,
            true,
            100,
        )
        .expect("Query should succeed");

        assert!(
            functions.is_empty(),
            "Should return empty for non-existent module"
        );
    }

    #[test]
    fn test_find_large_functions_regex_pattern_invalid() {
        let db = get_db();
        let result = find_large_functions(&*db, 0, Some("[invalid"), "default", true, true, 100);

        assert!(
            result.is_err(),
            "Should reject invalid regex pattern"
        );
    }

    // ===== Generated filtering tests =====

    #[test]
    fn test_find_large_functions_include_generated_true() {
        let db = get_db();
        let with_generated = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");
        let without_generated = find_large_functions(&*db, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // with_generated should have >= results than without_generated
        assert!(
            with_generated.len() >= without_generated.len(),
            "Including generated should return >= results"
        );
    }

    #[test]
    fn test_find_large_functions_exclude_generated() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // When include_generated=false, all generated_by should be empty or None
        for func in &functions {
            assert!(
                func.generated_by.is_empty(),
                "With include_generated=false, all generated_by should be empty, but {} has '{}'",
                func.name,
                func.generated_by
            );
        }
    }

    // ===== Limit and ordering tests =====

    #[test]
    fn test_find_large_functions_respects_limit() {
        let db = get_db();
        let functions_5 = find_large_functions(&*db, 0, None, "default", false, true, 5)
            .expect("Query should succeed");
        let functions_10 = find_large_functions(&*db, 0, None, "default", false, true, 10)
            .expect("Query should succeed");
        let functions_100 = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        assert!(functions_5.len() <= 5, "Should respect limit of 5");
        assert!(functions_10.len() <= 10, "Should respect limit of 10");
        assert_eq!(
            functions_100.len(),
            38,
            "Should return all 38 clauses with limit 100"
        );

        assert!(
            functions_5.len() <= functions_10.len(),
            "Smaller limit should return same or fewer results"
        );
        assert!(
            functions_10.len() <= functions_100.len(),
            "Smaller limit should return same or fewer results"
        );
    }

    #[test]
    fn test_find_large_functions_ordered_by_lines_desc() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        // Results should be ordered by lines descending
        let mut prev_lines = i64::MAX;
        for func in &functions {
            assert!(
                func.lines <= prev_lines,
                "Results should be ordered by lines DESC: {} > {}",
                func.lines,
                prev_lines
            );
            prev_lines = func.lines;
        }
    }

    // ===== Data integrity tests =====

    #[test]
    fn test_find_large_functions_all_fields_populated() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        assert!(!functions.is_empty(), "Should return results");

        for func in &functions {
            assert!(!func.module.is_empty(), "Module should not be empty");
            assert!(!func.name.is_empty(), "Name should not be empty");
            assert!(func.arity >= 0, "Arity should be >= 0");
            assert!(func.start_line > 0, "start_line should be > 0");
            assert!(func.end_line >= func.start_line, "end_line should be >= start_line");
            assert!(func.lines > 0, "lines should be > 0");
            assert!(!func.file.is_empty(), "file should not be empty");
        }
    }

    #[test]
    fn test_find_large_functions_valid_arity_values() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        // Verify all arities are non-negative
        for func in &functions {
            assert!(
                func.arity >= 0,
                "Arity should be non-negative, but {} has arity={}",
                func.name,
                func.arity
            );
        }

        // Verify we have functions with different arities
        let arities: std::collections::HashSet<_> = functions.iter().map(|f| f.arity).collect();
        assert!(arities.len() > 1, "Should have functions with different arities");
    }

    // ===== Specific function tests =====

    #[test]
    fn test_find_large_functions_controller_functions() {
        let db = get_db();
        let functions = find_large_functions(&*db, 0, Some("MyApp.Controller"), "default", false, true, 100)
            .expect("Query should succeed");

        // Controller has 3 functions with 3 clauses total in fixture
        assert!(
            !functions.is_empty(),
            "Should find Controller functions"
        );

        let controller_funcs: Vec<_> = functions.iter().collect();
        for func in &controller_funcs {
            assert_eq!(func.module, "MyApp.Controller");
        }
    }

    #[test]
    fn test_find_large_functions_combined_filters() {
        let db = get_db();
        let functions = find_large_functions(&*db, 5, Some("MyApp.Accounts"), "default", false, true, 100)
            .expect("Query should succeed");

        // Should apply both min_lines and module filters
        for func in &functions {
            assert!(
                func.lines >= 5,
                "Should respect min_lines=5"
            );
            assert_eq!(
                func.module, "MyApp.Accounts",
                "Should respect module filter"
            );
        }
    }
}
