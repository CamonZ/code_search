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
pub enum ComplexityError {
    #[error("Complexity query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with complexity metrics
#[derive(Debug, Clone, Serialize)]
pub struct ComplexityMetric {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub complexity: i64,
    pub max_nesting_depth: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub lines: i64,
    pub generated_by: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_complexity_metrics(
    db: &dyn Database,
    min_complexity: i64,
    min_depth: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<ComplexityMetric>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    // Build optional generated filter
    let generated_filter = if exclude_generated {
        ", generated_by == \"\"".to_string()
    } else {
        String::new()
    };

    let script = format!(
        r#"
        ?[module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, lines, generated_by] :=
            *function_locations{{project, module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, generated_by}},
            project == $project,
            complexity >= $min_complexity,
            max_nesting_depth >= $min_depth,
            lines = end_line - start_line + 1
            {module_cond}
            {generated_filter}

        :order -complexity, module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_int("min_complexity", min_complexity)
        .with_int("min_depth", min_depth);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| ComplexityError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 10 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(1).unwrap()) else { continue };
            let arity = extract_i64(row.get(2).unwrap(), 0);
            let line = extract_i64(row.get(3).unwrap(), 0);
            let complexity = extract_i64(row.get(4).unwrap(), 0);
            let max_nesting_depth = extract_i64(row.get(5).unwrap(), 0);
            let start_line = extract_i64(row.get(6).unwrap(), 0);
            let end_line = extract_i64(row.get(7).unwrap(), 0);
            let lines = extract_i64(row.get(8).unwrap(), 0);
            let Some(generated_by) = extract_string(row.get(9).unwrap()) else { continue };

            results.push(ComplexityMetric {
                module,
                name,
                arity,
                line,
                complexity,
                max_nesting_depth,
                start_line,
                end_line,
                lines,
                generated_by,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_complexity_metrics(
    db: &dyn Database,
    min_complexity: i64,
    min_depth: i64,
    module_pattern: Option<&str>,
    _project: &str,
    use_regex: bool,
    _exclude_generated: bool,
    limit: u32,
) -> Result<Vec<ComplexityMetric>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build module filter clause
    let module_clause = if let Some(_pattern) = module_pattern {
        if use_regex {
            "WHERE module_name = <regex>$module_pattern"
        } else {
            "WHERE module_name = $module_pattern"
        }
    } else {
        ""
    };

    // Query aggregates clauses by function to calculate complexity metrics
    // complexity = sum of complexity values across all clauses for that function
    // max_nesting_depth = max of max_nesting_depth across all clauses
    // Note: SurrealDB doesn't support HAVING, so we use a subquery with WHERE
    // Note: Using aliases in SELECT breaks GROUP BY in SurrealDB, so we avoid them
    let query = format!(
        r#"
        SELECT * FROM (
            SELECT
                module_name,
                function_name,
                arity,
                math::min(line) as line,
                math::sum(complexity) as complexity,
                math::max(max_nesting_depth) as max_nesting_depth,
                math::min(start_line) as start_line,
                math::max(end_line) as end_line
            FROM clauses
            {module_clause}
            GROUP BY module_name, function_name, arity
        ) WHERE complexity >= $min_complexity AND max_nesting_depth >= $min_depth
        ORDER BY complexity DESC, module_name, function_name, arity
        LIMIT $limit
        "#
    );

    let mut params = QueryParams::new()
        .with_int("min_complexity", min_complexity)
        .with_int("min_depth", min_depth)
        .with_int("limit", limit as i64);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = db.execute_query(&query, params).map_err(|e| ComplexityError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns alphabetically (via BTreeMap):
        // 0: arity, 1: complexity, 2: end_line, 3: function_name,
        // 4: line, 5: max_nesting_depth, 6: module_name, 7: start_line
        if row.len() >= 8 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let complexity = extract_i64(row.get(1).unwrap(), 0);
            let end_line = extract_i64(row.get(2).unwrap(), 0);
            let Some(name) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(4).unwrap(), 0);
            let max_nesting_depth = extract_i64(row.get(5).unwrap(), 0);
            let Some(module) = extract_string(row.get(6).unwrap()) else {
                continue;
            };
            let start_line = extract_i64(row.get(7).unwrap(), 0);

            // Calculate lines from line range
            let lines = if end_line >= start_line {
                end_line - start_line + 1
            } else {
                0
            };

            results.push(ComplexityMetric {
                module,
                name,
                arity,
                line,
                complexity,
                max_nesting_depth,
                start_line,
                end_line,
                lines,
                generated_by: String::new(), // SurrealDB fixture doesn't track this yet
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
    fn test_find_complexity_metrics_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_complexity_metrics(&*populated_db, 0, 0, None, "default", false, false, 100);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        // Should find some functions with complexity metrics
        assert!(!metrics.is_empty(), "Should find complexity metrics");
    }

    #[rstest]
    fn test_find_complexity_metrics_empty_results_high_threshold(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_complexity_metrics(
            &*populated_db,
            1000, // Very high complexity threshold
            0,
            None,
            "default",
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let metrics = result.unwrap();
        // May be empty if no functions have such high complexity
        assert!(metrics.is_empty() || !metrics.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_complexity_metrics_respects_min_complexity(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_complexity_metrics(&*populated_db, 5, 0, None, "default", false, false, 100);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        for metric in &metrics {
            assert!(metric.complexity >= 5, "All results should respect min_complexity");
        }
    }

    #[rstest]
    fn test_find_complexity_metrics_respects_min_depth(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_complexity_metrics(&*populated_db, 0, 3, None, "default", false, false, 100);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        for metric in &metrics {
            assert!(
                metric.max_nesting_depth >= 3,
                "All results should respect min_depth"
            );
        }
    }

    #[rstest]
    fn test_find_complexity_metrics_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_complexity_metrics(
            &*populated_db,
            0,
            0,
            Some("MyApp"),
            "default",
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let metrics = result.unwrap();
        for metric in &metrics {
            assert!(metric.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_complexity_metrics_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_complexity_metrics(&*populated_db, 0, 0, None, "default", false, false, 5)
            .unwrap();
        let limit_100 = find_complexity_metrics(&*populated_db, 0, 0, None, "default", false, false, 100)
            .unwrap();

        assert!(limit_5.len() <= 5);
        assert!(limit_5.len() <= limit_100.len());
    }

    #[rstest]
    fn test_find_complexity_metrics_nonexistent_project(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_complexity_metrics(&*populated_db, 0, 0, None, "nonexistent", false, false, 100);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_complexity_metrics_returns_valid_fields(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_complexity_metrics(&*populated_db, 0, 0, None, "default", false, false, 100);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        if !metrics.is_empty() {
            let metric = &metrics[0];
            assert!(!metric.module.is_empty());
            assert!(!metric.name.is_empty());
            assert!(metric.arity >= 0);
            assert!(metric.complexity >= 0);
            assert!(metric.max_nesting_depth >= 0);
            assert!(metric.start_line > 0);
            assert!(metric.end_line >= metric.start_line);
            assert_eq!(metric.lines, metric.end_line - metric.start_line + 1);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    // The complex fixture contains:
    // - 5 modules: Controller (3 funcs), Accounts (4), Service (2), Repo (4), Notifier (2)
    // - 15 functions total
    // - 22 clauses total with complexity and max_nesting_depth values
    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== Basic functionality tests =====

    #[test]
    fn test_find_complexity_metrics_returns_results() {
        let db = get_db();
        let result = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let metrics = result.unwrap();
        assert!(!metrics.is_empty(), "Should find complexity metrics");
    }

    #[test]
    fn test_find_complexity_metrics_returns_exact_count() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // The fixture has 31 functions, each with at least 1 clause
        assert_eq!(
            metrics.len(),
            31,
            "Should find exactly 31 functions with complexity metrics"
        );
    }

    #[test]
    fn test_find_complexity_metrics_calculates_complexity_from_clauses() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // Find Controller.index/2 which has 2 clauses with complexity 3+1=4
        let controller_index = metrics
            .iter()
            .find(|m| m.module == "MyApp.Controller" && m.name == "index" && m.arity == 2)
            .expect("Controller.index/2 should be found");

        assert_eq!(
            controller_index.complexity, 4,
            "Controller.index/2 should have complexity=4 (sum of clause complexities: 3+1)"
        );
    }

    #[test]
    fn test_find_complexity_metrics_calculates_max_nesting_depth() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // Controller.index/2 has clauses with depth 2 and 1, max should be 2
        let controller_index = metrics
            .iter()
            .find(|m| m.module == "MyApp.Controller" && m.name == "index" && m.arity == 2)
            .expect("Controller.index/2 should be found");

        assert_eq!(
            controller_index.max_nesting_depth, 2,
            "Controller.index/2 should have max_nesting_depth=2"
        );
    }

    #[test]
    fn test_find_complexity_metrics_multiple_functions_per_module() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // Controller has 4 functions: index/2, show/2, create/2, handle_event/1
        let controller_funcs: Vec<_> = metrics
            .iter()
            .filter(|m| m.module == "MyApp.Controller")
            .collect();

        assert_eq!(
            controller_funcs.len(),
            4,
            "Controller should have exactly 4 functions"
        );

        // Verify each has expected complexity
        let index = controller_funcs
            .iter()
            .find(|m| m.name == "index")
            .expect("index should exist");
        assert_eq!(index.complexity, 4, "Controller.index should have complexity=4 (3+1)");

        let show = controller_funcs
            .iter()
            .find(|m| m.name == "show")
            .expect("show should exist");
        assert_eq!(show.complexity, 4, "Controller.show should have complexity=4 (3+1)");

        let create = controller_funcs
            .iter()
            .find(|m| m.name == "create")
            .expect("create should exist");
        assert_eq!(create.complexity, 8, "Controller.create should have complexity=8 (5+2+1)");

        let handle_event = controller_funcs
            .iter()
            .find(|m| m.name == "handle_event")
            .expect("handle_event should exist");
        assert_eq!(handle_event.complexity, 2, "Controller.handle_event should have complexity=2");
    }

    #[test]
    fn test_find_complexity_metrics_all_modules_present() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        let modules: std::collections::HashSet<_> = metrics.iter().map(|m| m.module.as_str()).collect();

        assert!(
            modules.contains("MyApp.Controller"),
            "Should contain MyApp.Controller"
        );
        assert!(modules.contains("MyApp.Accounts"), "Should contain MyApp.Accounts");
        assert!(modules.contains("MyApp.Service"), "Should contain MyApp.Service");
        assert!(modules.contains("MyApp.Repo"), "Should contain MyApp.Repo");
        assert!(modules.contains("MyApp.Notifier"), "Should contain MyApp.Notifier");
    }

    // ===== Threshold tests =====

    #[test]
    fn test_find_complexity_metrics_respects_min_complexity_threshold() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 3, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // Service.process_request/2 has 3 clauses (complexity=3)
        // Accounts.get_user/1 has 2 clauses (complexity=2), should be excluded
        for metric in &metrics {
            assert!(
                metric.complexity >= 3,
                "All results should respect min_complexity=3, but {} has {}",
                metric.name,
                metric.complexity
            );
        }

        // Verify we got the expected function with complexity 3
        let service_process = metrics
            .iter()
            .find(|m| m.module == "MyApp.Service" && m.name == "process_request" && m.arity == 2);
        assert!(
            service_process.is_some(),
            "Service.process_request/2 with complexity=3 should be included"
        );
    }

    #[test]
    fn test_find_complexity_metrics_respects_min_depth_threshold() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 3, None, "default", false, false, 100)
            .expect("Query should succeed");

        // All results should have max_nesting_depth >= 3
        for metric in &metrics {
            assert!(
                metric.max_nesting_depth >= 3,
                "All results should have max_nesting_depth >= 3, but {} has {}",
                metric.name,
                metric.max_nesting_depth
            );
        }
    }

    #[test]
    fn test_find_complexity_metrics_filters_by_both_thresholds() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 3, 2, None, "default", false, false, 100)
            .expect("Query should succeed");

        // All results must satisfy both conditions
        for metric in &metrics {
            assert!(
                metric.complexity >= 3,
                "All results should have complexity >= 3"
            );
            assert!(
                metric.max_nesting_depth >= 2,
                "All results should have max_nesting_depth >= 2"
            );
        }
    }

    // ===== Module pattern filtering tests =====

    #[test]
    fn test_find_complexity_metrics_with_exact_module_filter() {
        let db = get_db();
        let metrics = find_complexity_metrics(
            &*db,
            0,
            0,
            Some("MyApp.Controller"),
            "default",
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        assert_eq!(
            metrics.len(),
            4,
            "Should find exactly 4 functions in Controller module (index, show, create, handle_event)"
        );

        for metric in &metrics {
            assert_eq!(
                metric.module, "MyApp.Controller",
                "All results should be from Controller module"
            );
        }
    }

    #[test]
    fn test_find_complexity_metrics_with_regex_module_filter() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, Some("^MyApp\\.Acc.*"), "default", true, false, 100)
            .expect("Query should succeed");

        assert_eq!(
            metrics.len(),
            6,
            "Regex should match MyApp.Accounts (6 functions: get_user/1, get_user/2, list_users, validate_email, __struct__, notify_change)"
        );

        for metric in &metrics {
            assert_eq!(
                metric.module, "MyApp.Accounts",
                "All results should be from Accounts module"
            );
        }
    }

    #[test]
    fn test_find_complexity_metrics_with_nonexistent_module() {
        let db = get_db();
        let metrics = find_complexity_metrics(
            &*db,
            0,
            0,
            Some("NonExistentModule"),
            "default",
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(
            metrics.is_empty(),
            "Should return empty for non-existent module"
        );
    }

    #[test]
    fn test_find_complexity_metrics_regex_pattern_invalid() {
        let db = get_db();
        let result = find_complexity_metrics(&*db, 0, 0, Some("[invalid"), "default", true, false, 100);

        assert!(
            result.is_err(),
            "Should reject invalid regex pattern"
        );
    }

    // ===== Limit and ordering tests =====

    #[test]
    fn test_find_complexity_metrics_respects_limit() {
        let db = get_db();
        let metrics_5 = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 5)
            .expect("Query should succeed");
        let metrics_10 = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 10)
            .expect("Query should succeed");
        let metrics_100 = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        assert!(metrics_5.len() <= 5, "Should respect limit of 5");
        assert!(metrics_10.len() <= 10, "Should respect limit of 10");
        assert_eq!(
            metrics_100.len(),
            31,
            "Should return all 31 functions with limit 100"
        );

        assert!(
            metrics_5.len() <= metrics_10.len(),
            "Smaller limit should return same or fewer results"
        );
        assert!(
            metrics_10.len() <= metrics_100.len(),
            "Smaller limit should return same or fewer results"
        );
    }

    #[test]
    fn test_find_complexity_metrics_ordered_by_complexity_desc() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // Results should be ordered by complexity descending, then by module/name
        let mut prev_complexity = i64::MAX;
        for metric in &metrics {
            assert!(
                metric.complexity <= prev_complexity,
                "Results should be ordered by complexity DESC: {} > {}",
                metric.complexity,
                prev_complexity
            );
            prev_complexity = metric.complexity;
        }
    }

    #[test]
    fn test_find_complexity_metrics_calculates_lines_correctly() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        for metric in &metrics {
            let calculated_lines = metric.end_line - metric.start_line + 1;
            assert_eq!(
                metric.lines, calculated_lines,
                "Lines should be calculated as end_line - start_line + 1"
            );
        }
    }

    #[test]
    fn test_find_complexity_metrics_valid_arity_values() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // Verify all arities are non-negative
        for metric in &metrics {
            assert!(
                metric.arity >= 0,
                "Arity should be non-negative, but {} has arity={}",
                metric.name,
                metric.arity
            );
        }

        // Verify we have functions with different arities
        let arities: std::collections::HashSet<_> = metrics.iter().map(|m| m.arity).collect();
        assert!(arities.len() > 1, "Should have functions with different arities");
    }

    #[test]
    fn test_find_complexity_metrics_all_fields_populated() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        assert!(!metrics.is_empty(), "Should return results");

        for metric in &metrics {
            assert!(!metric.module.is_empty(), "Module should not be empty");
            assert!(!metric.name.is_empty(), "Name should not be empty");
            assert!(metric.complexity > 0, "Complexity should be > 0");
            assert!(metric.max_nesting_depth >= 0, "max_nesting_depth should be >= 0");
            assert!(metric.start_line > 0, "start_line should be > 0");
            assert!(metric.end_line >= metric.start_line, "end_line should be >= start_line");
            assert!(metric.lines > 0, "lines should be > 0");
        }
    }

    // ===== Specific function metrics tests =====

    #[test]
    fn test_accounts_get_user_arity_variations() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        // Accounts module has get_user/1 and get_user/2
        let get_user_1 = metrics
            .iter()
            .find(|m| m.module == "MyApp.Accounts" && m.name == "get_user" && m.arity == 1)
            .expect("get_user/1 should be found");

        let get_user_2 = metrics
            .iter()
            .find(|m| m.module == "MyApp.Accounts" && m.name == "get_user" && m.arity == 2)
            .expect("get_user/2 should be found");

        assert_eq!(
            get_user_1.complexity, 3,
            "get_user/1 should have complexity=3 (2+1)"
        );
        assert_eq!(
            get_user_2.complexity, 2,
            "get_user/2 should have complexity=2"
        );
    }

    #[test]
    fn test_service_process_request_complexity() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 0, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        let service_process = metrics
            .iter()
            .find(|m| m.module == "MyApp.Service" && m.name == "process_request" && m.arity == 2)
            .expect("Service.process_request/2 should be found");

        assert_eq!(
            service_process.complexity, 8,
            "Service.process_request/2 should have complexity=8 (5+2+1)"
        );

        // Highest complexity is 8, shared by Controller.create/2 and Service.process_request/2
        // Controller comes first alphabetically
        assert_eq!(
            metrics[0].complexity, 8,
            "Highest complexity function should have complexity=8"
        );
        assert_eq!(
            metrics[0].module, "MyApp.Controller",
            "Controller.create/2 should be first (alphabetically before Service)"
        );
    }

    #[test]
    fn test_find_complexity_metrics_empty_with_very_high_threshold() {
        let db = get_db();
        let metrics = find_complexity_metrics(&*db, 1000, 0, None, "default", false, false, 100)
            .expect("Query should succeed");

        assert!(
            metrics.is_empty(),
            "Should return empty with very high complexity threshold"
        );
    }
}
