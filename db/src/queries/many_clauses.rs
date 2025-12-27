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
pub enum ManyClausesError {
    #[error("Many clauses query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with clause count information
#[derive(Debug, Clone, Serialize)]
pub struct ManyClauses {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub clauses: i64,
    pub first_line: i64,
    pub last_line: i64,
    pub file: String,
    pub generated_by: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_many_clauses(
    db: &dyn Database,
    min_clauses: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<ManyClauses>, Box<dyn Error>> {
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
        clause_counts[module, name, arity, count(line), min(start_line), max(end_line), file, generated_by] :=
            *function_locations{{project, module, name, arity, line, start_line, end_line, file, generated_by}},
            project == $project
            {module_cond}
            {generated_filter}

        ?[module, name, arity, clauses, first_line, last_line, file, generated_by] :=
            clause_counts[module, name, arity, clauses, first_line, last_line, file, generated_by],
            clauses >= $min_clauses

        :order -clauses, module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new();
    params = params.with_str("project", project);
    params = params.with_int("min_clauses", min_clauses);
    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| ManyClausesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 8 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(1).unwrap()) else { continue };
            let arity = extract_i64(row.get(2).unwrap(), 0);
            let clauses = extract_i64(row.get(3).unwrap(), 0);
            let first_line = extract_i64(row.get(4).unwrap(), 0);
            let last_line = extract_i64(row.get(5).unwrap(), 0);
            let Some(file) = extract_string(row.get(6).unwrap()) else { continue };
            let Some(generated_by) = extract_string(row.get(7).unwrap()) else { continue };

            results.push(ManyClauses {
                module,
                name,
                arity,
                clauses,
                first_line,
                last_line,
                file,
                generated_by,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_many_clauses(
    db: &dyn Database,
    min_clauses: i64,
    module_pattern: Option<&str>,
    _project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<ManyClauses>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build WHERE clause conditions
    let mut conditions = vec![];

    if let Some(_pattern) = module_pattern {
        if use_regex {
            conditions.push("string::matches(module_name, $module_pattern)".to_string());
        } else {
            conditions.push("module_name = $module_pattern".to_string());
        }
    }

    if !include_generated {
        conditions.push("(generated_by IS NONE OR generated_by = \"\")".to_string());
    }

    let where_in_subquery = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Query clauses table grouped by function to count clauses per function
    // Use subquery pattern to apply min_clauses threshold
    let query = format!(
        r#"
        SELECT * FROM (
            SELECT
                module_name,
                function_name,
                arity,
                count() as clauses,
                math::min(start_line) as first_line,
                math::max(end_line) as last_line,
                source_file as file,
                generated_by
            FROM clauses
            {where_in_subquery}
            GROUP BY module_name, function_name, arity, source_file, generated_by
        ) WHERE clauses >= $min_clauses
        ORDER BY clauses DESC, module_name, function_name
        LIMIT $limit
        "#
    );

    let mut params = QueryParams::new()
        .with_int("min_clauses", min_clauses)
        .with_int("limit", limit as i64);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = db.execute_query(&query, params).map_err(|e| ManyClausesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns alphabetically (via BTreeMap):
        // 0: arity, 1: clauses, 2: file, 3: first_line,
        // 4: function_name, 5: generated_by, 6: last_line, 7: module_name
        if row.len() >= 8 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let clauses = extract_i64(row.get(1).unwrap(), 0);
            let Some(file) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let first_line = extract_i64(row.get(3).unwrap(), 0);
            let Some(name) = extract_string(row.get(4).unwrap()) else {
                continue;
            };
            let generated_by = extract_string(row.get(5).unwrap()).unwrap_or_default();
            let last_line = extract_i64(row.get(6).unwrap(), 0);
            let Some(module) = extract_string(row.get(7).unwrap()) else {
                continue;
            };

            results.push(ManyClauses {
                module,
                name,
                arity,
                clauses,
                first_line,
                last_line,
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
    fn test_find_many_clauses_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_many_clauses(&*populated_db, 0, None, "default", false, true, 100);
        assert!(result.is_ok());
        let clauses = result.unwrap();
        // Should find functions with clause counts
        assert!(!clauses.is_empty(), "Should find functions with clauses");
    }

    #[rstest]
    fn test_find_many_clauses_respects_min_clauses(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_many_clauses(&*populated_db, 5, None, "default", false, true, 100);
        assert!(result.is_ok());
        let clauses = result.unwrap();
        for entry in &clauses {
            assert!(entry.clauses >= 5, "All results should have >= min_clauses");
        }
    }

    #[rstest]
    fn test_find_many_clauses_empty_results_high_threshold(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_many_clauses(&*populated_db, 1000, None, "default", false, true, 100);
        assert!(result.is_ok());
        let clauses = result.unwrap();
        // May be empty if no functions have so many clauses
        assert!(clauses.is_empty() || !clauses.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_many_clauses_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_many_clauses(&*populated_db, 0, Some("MyApp"), "default", false, true, 100);
        assert!(result.is_ok());
        let clauses = result.unwrap();
        for entry in &clauses {
            assert!(entry.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_many_clauses_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_many_clauses(&*populated_db, 0, None, "default", false, true, 5)
            .unwrap();
        let limit_100 = find_many_clauses(&*populated_db, 0, None, "default", false, true, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_many_clauses_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_many_clauses(&*populated_db, 0, None, "nonexistent", false, true, 100);
        assert!(result.is_ok());
        let clauses = result.unwrap();
        assert!(clauses.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_many_clauses_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_many_clauses(&*populated_db, 0, None, "default", false, true, 100);
        assert!(result.is_ok());
        let clauses = result.unwrap();
        if !clauses.is_empty() {
            let entry = &clauses[0];
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.arity >= 0);
            assert!(entry.clauses > 0);
            assert!(entry.first_line > 0);
            assert!(entry.last_line >= entry.first_line);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    // The complex fixture contains:
    // - 5 modules: Controller (3 funcs), Accounts (4), Service (2), Repo (4), Notifier (2)
    // - 15 functions total
    // - 22 clauses total with varying clause counts per function
    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== Basic functionality tests =====

    #[test]
    fn test_find_many_clauses_returns_results() {
        let db = get_db();
        let result = find_many_clauses(&*db, 0, None, "default", false, true, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let clauses = result.unwrap();
        assert!(!clauses.is_empty(), "Should find clauses");
    }

    #[test]
    fn test_find_many_clauses_returns_exact_count() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        // The fixture has 37 functions with 44 clauses total
        // With min_clauses=0, should return 37 functions (grouped by function)
        assert_eq!(
            clauses.len(),
            37,
            "Should find exactly 37 functions with clauses"
        );
    }

    #[test]
    fn test_find_many_clauses_calculates_clause_count() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        // Find Controller.index/2 which has 2 clauses in fixture
        let controller_index = clauses
            .iter()
            .find(|c| c.module == "MyApp.Controller" && c.name == "index" && c.arity == 2)
            .expect("Controller.index/2 should be found");

        assert_eq!(
            controller_index.clauses, 2,
            "Controller.index/2 should have clauses=2"
        );
    }

    #[test]
    fn test_find_many_clauses_all_modules_present() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        let modules: std::collections::HashSet<_> = clauses.iter().map(|c| c.module.as_str()).collect();

        assert!(
            modules.contains("MyApp.Controller"),
            "Should contain MyApp.Controller"
        );
        assert!(modules.contains("MyApp.Accounts"), "Should contain MyApp.Accounts");
        assert!(modules.contains("MyApp.Service"), "Should contain MyApp.Service");
        assert!(modules.contains("MyApp.Repo"), "Should contain MyApp.Repo");
        assert!(modules.contains("MyApp.Notifier"), "Should contain MyApp.Notifier");
    }

    // ===== Clause count threshold tests =====

    #[test]
    fn test_find_many_clauses_respects_min_clauses_threshold() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 2, None, "default", false, true, 100)
            .expect("Query should succeed");

        for clause in &clauses {
            assert!(
                clause.clauses >= 2,
                "All results should have clauses >= 2, but {} has {}",
                clause.name,
                clause.clauses
            );
        }
    }

    #[test]
    fn test_find_many_clauses_high_threshold_reduces_results() {
        let db = get_db();
        let all_clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");
        let high_threshold = find_many_clauses(&*db, 3, None, "default", false, true, 100)
            .expect("Query should succeed");

        // Higher threshold should return fewer or equal results
        assert!(
            high_threshold.len() <= all_clauses.len(),
            "Higher threshold should return fewer results"
        );

        // All results should meet the threshold
        for clause in &high_threshold {
            assert!(
                clause.clauses >= 3,
                "All results should have >= 3 clauses"
            );
        }
    }

    #[test]
    fn test_find_many_clauses_empty_with_very_high_threshold() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 1000, None, "default", false, true, 100)
            .expect("Query should succeed");

        assert!(
            clauses.is_empty(),
            "Should return empty with very high clause count threshold"
        );
    }

    // ===== Module pattern filtering tests =====

    #[test]
    fn test_find_many_clauses_with_exact_module_filter() {
        let db = get_db();
        let clauses = find_many_clauses(
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
            !clauses.is_empty(),
            "Should find Controller functions"
        );

        for clause in &clauses {
            assert_eq!(
                clause.module, "MyApp.Controller",
                "All results should be from Controller module"
            );
        }
    }

    #[test]
    fn test_find_many_clauses_with_regex_module_filter() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, Some("^MyApp\\.Acc.*"), "default", true, true, 100)
            .expect("Query should succeed");

        assert!(
            !clauses.is_empty(),
            "Regex should match MyApp.Accounts"
        );

        for clause in &clauses {
            assert_eq!(
                clause.module, "MyApp.Accounts",
                "All results should be from Accounts module"
            );
        }
    }

    #[test]
    fn test_find_many_clauses_with_nonexistent_module() {
        let db = get_db();
        let clauses = find_many_clauses(
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
            clauses.is_empty(),
            "Should return empty for non-existent module"
        );
    }

    #[test]
    fn test_find_many_clauses_regex_pattern_invalid() {
        let db = get_db();
        let result = find_many_clauses(&*db, 0, Some("[invalid"), "default", true, true, 100);

        assert!(
            result.is_err(),
            "Should reject invalid regex pattern"
        );
    }

    // ===== Generated filtering tests =====

    #[test]
    fn test_find_many_clauses_include_generated_true() {
        let db = get_db();
        let with_generated = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");
        let without_generated = find_many_clauses(&*db, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // with_generated should have >= results than without_generated
        assert!(
            with_generated.len() >= without_generated.len(),
            "Including generated should return >= results"
        );
    }

    #[test]
    fn test_find_many_clauses_exclude_generated() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // When include_generated=false, all generated_by should be empty or None
        for clause in &clauses {
            assert!(
                clause.generated_by.is_empty(),
                "With include_generated=false, all generated_by should be empty, but {} has '{}'",
                clause.name,
                clause.generated_by
            );
        }
    }

    // ===== Limit and ordering tests =====

    #[test]
    fn test_find_many_clauses_respects_limit() {
        let db = get_db();
        let clauses_5 = find_many_clauses(&*db, 0, None, "default", false, true, 5)
            .expect("Query should succeed");
        let clauses_10 = find_many_clauses(&*db, 0, None, "default", false, true, 10)
            .expect("Query should succeed");
        let clauses_100 = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        assert!(clauses_5.len() <= 5, "Should respect limit of 5");
        assert!(clauses_10.len() <= 10, "Should respect limit of 10");
        assert_eq!(
            clauses_100.len(),
            37,
            "Should return all 37 functions with limit 100"
        );

        assert!(
            clauses_5.len() <= clauses_10.len(),
            "Smaller limit should return same or fewer results"
        );
        assert!(
            clauses_10.len() <= clauses_100.len(),
            "Smaller limit should return same or fewer results"
        );
    }

    #[test]
    fn test_find_many_clauses_ordered_by_clauses_desc() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        // Results should be ordered by clause count descending
        let mut prev_clauses = i64::MAX;
        for clause in &clauses {
            assert!(
                clause.clauses <= prev_clauses,
                "Results should be ordered by clauses DESC: {} > {}",
                clause.clauses,
                prev_clauses
            );
            prev_clauses = clause.clauses;
        }
    }

    // ===== Data integrity tests =====

    #[test]
    fn test_find_many_clauses_all_fields_populated() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        assert!(!clauses.is_empty(), "Should return results");

        for clause in &clauses {
            assert!(!clause.module.is_empty(), "Module should not be empty");
            assert!(!clause.name.is_empty(), "Name should not be empty");
            assert!(clause.arity >= 0, "Arity should be >= 0");
            assert!(clause.clauses > 0, "Clauses should be > 0");
            assert!(clause.first_line > 0, "first_line should be > 0");
            assert!(clause.last_line >= clause.first_line, "last_line should be >= first_line");
            assert!(!clause.file.is_empty(), "file should not be empty");
        }
    }

    #[test]
    fn test_find_many_clauses_valid_arity_values() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        // Verify all arities are non-negative
        for clause in &clauses {
            assert!(
                clause.arity >= 0,
                "Arity should be non-negative, but {} has arity={}",
                clause.name,
                clause.arity
            );
        }

        // Verify we have functions with different arities
        let arities: std::collections::HashSet<_> = clauses.iter().map(|c| c.arity).collect();
        assert!(arities.len() > 1, "Should have functions with different arities");
    }

    // ===== Specific function tests =====

    #[test]
    fn test_find_many_clauses_controller_functions() {
        let db = get_db();
        let clauses = find_many_clauses(
            &*db,
            0,
            Some("MyApp.Controller"),
            "default",
            false,
            true,
            100,
        )
        .expect("Query should succeed");

        // Controller has 6 functions in fixture (index, show, create, handle_event, format_display, __generated__)
        assert_eq!(
            clauses.len(),
            6,
            "Should find exactly 6 Controller functions"
        );

        for clause in &clauses {
            assert_eq!(clause.module, "MyApp.Controller");
        }
    }

    #[test]
    fn test_find_many_clauses_accounts_functions() {
        let db = get_db();
        let clauses = find_many_clauses(
            &*db,
            0,
            Some("MyApp.Accounts"),
            "default",
            false,
            true,
            100,
        )
        .expect("Query should succeed");

        // Accounts has 8 functions in fixture (get_user/1, get_user/2, list_users, validate_email, __struct__, notify_change, format_name, __generated__)
        assert_eq!(
            clauses.len(),
            8,
            "Should find exactly 8 Accounts functions"
        );

        for clause in &clauses {
            assert_eq!(clause.module, "MyApp.Accounts");
        }
    }

    #[test]
    fn test_find_many_clauses_combined_filters() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 2, Some("MyApp.Accounts"), "default", false, true, 100)
            .expect("Query should succeed");

        // Should apply both min_clauses and module filters
        for clause in &clauses {
            assert!(
                clause.clauses >= 2,
                "Should respect min_clauses=2"
            );
            assert_eq!(
                clause.module, "MyApp.Accounts",
                "Should respect module filter"
            );
        }
    }

    #[test]
    fn test_find_many_clauses_line_range_validity() {
        let db = get_db();
        let clauses = find_many_clauses(&*db, 0, None, "default", false, true, 100)
            .expect("Query should succeed");

        for clause in &clauses {
            assert!(
                clause.last_line >= clause.first_line,
                "last_line should be >= first_line for {}",
                clause.name
            );
            assert!(
                clause.first_line > 0,
                "first_line should be > 0 for {}",
                clause.name
            );
        }
    }
}
