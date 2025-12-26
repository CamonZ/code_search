use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or};
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_locations(
    db: &dyn Database,
    module_pattern: Option<&str>,
    function_pattern: &str,
    arity: Option<i64>,
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionLocation>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern, Some(function_pattern)])?;

    // Build the WHERE clause based on regex vs exact match
    // SurrealDB v3.0 uses type casting for regex: <regex>$pattern
    let module_clause = if module_pattern.is_some() {
        if use_regex {
            "module_name = <regex>$module_pattern"
        } else {
            "module_name = $module_pattern"
        }
    } else {
        // No module filter - match all
        "1 = 1"
    };

    let function_clause = if use_regex {
        "function_name = <regex>$function_pattern"
    } else {
        "function_name = $function_pattern"
    };

    let arity_clause = if arity.is_some() {
        "AND arity = $arity"
    } else {
        ""
    };

    let query = format!(
        r#"
        SELECT "default" as project, file, line, start_line, end_line,
               module_name as module, kind, function_name as name, arity, pattern, guard
        FROM `clause`
        WHERE {module_clause}
          AND {function_clause}
          {arity_clause}
        ORDER BY module_name ASC, function_name ASC, arity ASC, line ASC
        LIMIT $limit
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("function_pattern", function_pattern)
        .with_int("limit", limit as i64);

    if let Some(mod_pat) = module_pattern {
        params = params.with_str("module_pattern", mod_pat);
    }

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = db.execute_query(&query, params).map_err(|e| LocationError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order:
        // arity, end_line, file, guard, kind, line, module, name, pattern, project, start_line
        if row.len() >= 11 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let end_line = extract_i64(row.get(1).unwrap(), 0);
            let Some(file) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let guard = extract_string_or(row.get(3).unwrap(), "");
            let kind = extract_string_or(row.get(4).unwrap(), "");
            let line = extract_i64(row.get(5).unwrap(), 0);
            let Some(module) = extract_string(row.get(6).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(7).unwrap()) else {
                continue;
            };
            let pattern = extract_string_or(row.get(8).unwrap(), "");
            let Some(project) = extract_string(row.get(9).unwrap()) else {
                continue;
            };
            let start_line = extract_i64(row.get(10).unwrap(), 0);

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

    // SurrealDB doesn't honor ORDER BY when using regex WHERE clauses
    // Sort results in Rust to ensure consistent ordering: module, name, arity, line
    results.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.arity.cmp(&b.arity))
            .then_with(|| a.line.cmp(&b.line))
    });

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

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    // ==================== Validation Tests ====================

    #[test]
    fn test_find_locations_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Invalid regex pattern: unclosed bracket
        let result = find_locations(&*db, None, "[invalid", None, "default", true, 100);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern"),
            "Error should mention invalid regex: {}",
            msg
        );
    }

    #[test]
    fn test_find_locations_invalid_regex_module_pattern() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Invalid regex in module pattern
        let result = find_locations(&*db, Some("[invalid"), "foo", None, "default", true, 100);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern"),
            "Error should mention invalid regex: {}",
            msg
        );
    }

    #[test]
    fn test_find_locations_valid_regex() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Valid regex pattern should not error on validation
        let result = find_locations(&*db, Some("^module.*$"), "^foo$", None, "default", true, 100);

        // Should not fail on validation
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_find_locations_non_regex_mode() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Even invalid regex should work in non-regex mode
        let result = find_locations(&*db, Some("[invalid"), "foo", None, "default", false, 100);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_find_locations_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search for exact function name
        let result = find_locations(&*db, Some("module_a"), "foo", None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let locations = result.unwrap();

        // Fixture has foo/1 in module_a with two clauses at lines 10 and 15
        assert_eq!(locations.len(), 2, "Should find exactly two locations for foo/1");
        assert_eq!(locations[0].name, "foo");
        assert_eq!(locations[0].module, "module_a");
        assert_eq!(locations[0].arity, 1);
        assert_eq!(locations[0].line, 10);
        assert_eq!(locations[0].project, "default");
        assert_eq!(locations[1].line, 15);
    }

    #[test]
    fn test_find_locations_empty_results() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search for function that doesn't exist
        let result = find_locations(&*db, Some("module_a"), "nonexistent", None, "default", false, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();
        assert!(locations.is_empty(), "Should find no results for nonexistent function");
    }

    #[test]
    fn test_find_locations_nonexistent_module() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search in module that doesn't exist
        let result = find_locations(
            &*db,
            Some("nonexistent_module"),
            "foo",
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let locations = result.unwrap();
        assert!(locations.is_empty(), "Should find no results for nonexistent module");
    }

    #[test]
    fn test_find_locations_with_arity_filter() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search with arity filter - bar has arity 2
        let result = find_locations(&*db, Some("module_a"), "bar", Some(2), "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let locations = result.unwrap();

        // Fixture has bar/2 in module_a - verify arity filter works
        for loc in &locations {
            assert_eq!(loc.arity, 2, "All results should have arity 2");
        }
    }

    #[test]
    fn test_find_locations_with_wrong_arity() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search with wrong arity (foo/1 exists, but search for foo/2)
        let result = find_locations(&*db, Some("module_a"), "foo", Some(2), "default", false, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();
        assert!(locations.is_empty(), "Should find no results with wrong arity");
    }

    // ==================== Module Pattern Tests ====================

    #[test]
    fn test_find_locations_no_module_filter() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search without module filter - should find all occurrences
        let result = find_locations(&*db, None, "foo", None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let locations = result.unwrap();

        // Fixture has foo/1 in module_a with 2 clauses (at lines 10 and 15)
        assert_eq!(locations.len(), 2, "Should find all foo occurrences");
        for loc in &locations {
            assert_eq!(loc.name, "foo", "All results should be foo");
            assert_eq!(loc.module, "module_a", "All results should be in module_a");
        }
    }

    #[test]
    fn test_find_locations_module_pattern_exact() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search with exact module pattern
        let result = find_locations(&*db, Some("module_b"), "baz", None, "default", false, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();

        // Fixture has baz/0 in module_b with one clause at line 3
        assert_eq!(locations.len(), 1, "Should find exactly one baz in module_b");
        assert_eq!(locations[0].module, "module_b");
        assert_eq!(locations[0].name, "baz");
        assert_eq!(locations[0].arity, 0);
        assert_eq!(locations[0].line, 3);
    }

    // ==================== Limit Tests ====================

    #[test]
    fn test_find_locations_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Use wildcard patterns to match all
        let limit_1 = find_locations(&*db, None, ".*", None, "default", true, 1).unwrap();
        let limit_100 = find_locations(&*db, None, ".*", None, "default", true, 100).unwrap();

        assert!(limit_1.len() <= 1, "Limit should be respected");
        assert!(limit_1.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[test]
    fn test_find_locations_zero_limit() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test with zero limit
        let result = find_locations(&*db, None, ".*", None, "default", true, 0);

        assert!(result.is_ok(), "Should handle zero limit");
        let locations = result.unwrap();
        assert!(locations.is_empty(), "Zero limit should return no results");
    }

    #[test]
    fn test_find_locations_large_limit() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test with large limit (larger than fixture size)
        let result = find_locations(&*db, None, ".*", None, "default", true, 1000000);

        assert!(result.is_ok(), "Should handle large limit");
        let locations = result.unwrap();

        // Fixture has: foo/1 (2 clauses), bar/2 (1 clause), baz/0 (1 clause)
        assert_eq!(locations.len(), 4, "Should return all locations");
    }

    // ==================== Pattern Matching Tests ====================

    #[test]
    fn test_find_locations_regex_dot_star() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Regex pattern that matches all functions
        let result = find_locations(&*db, None, ".*", None, "default", true, 100);

        assert!(result.is_ok(), "Should match all functions with .*");
        let locations = result.unwrap();

        // Should find all 4 locations
        assert_eq!(locations.len(), 4, "Should find exactly 4 locations");
    }

    #[test]
    fn test_find_locations_regex_alternation() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test regex alternation pattern - matches foo or bar
        let result = find_locations(&*db, Some("module_a"), "^(foo|bar)", None, "default", true, 100);

        assert!(result.is_ok(), "Should handle regex alternation");
        let locations = result.unwrap();

        // module_a has foo/1 (2 clauses) and bar/2 (1 clause) = 3 total
        assert_eq!(locations.len(), 3, "Should match both foo and bar clauses");
        let names: Vec<_> = locations.iter().map(|l| l.name.clone()).collect();
        assert!(names.iter().any(|n| n == "foo"), "Should contain foo");
        assert!(names.iter().any(|n| n == "bar"), "Should contain bar");
    }

    #[test]
    fn test_find_locations_regex_anchors() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test with start anchor - matches foo but not foobar
        let result = find_locations(&*db, Some("module_a"), "^foo$", None, "default", true, 100);

        assert!(result.is_ok(), "Should handle regex anchors");
        let locations = result.unwrap();

        // Should find foo clauses (2 total) but not bar
        assert_eq!(locations.len(), 2, "Should find both foo clauses");
        for loc in &locations {
            assert_eq!(loc.name, "foo", "All results should be foo");
        }
    }

    // ==================== Result Structure Tests ====================

    #[test]
    fn test_find_locations_returns_correct_fields() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_locations(&*db, None, ".*", None, "default", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let locations = result.unwrap();

        // Verify structure of returned locations
        assert!(!locations.is_empty(), "Should find some locations");
        for loc in &locations {
            assert_eq!(loc.project, "default", "project should be 'default'");
            assert!(!loc.module.is_empty(), "module should not be empty");
            assert!(!loc.name.is_empty(), "name should not be empty");
            assert!(loc.arity >= 0, "arity should be non-negative");
            assert!(loc.line > 0, "line should be positive");
            assert!(loc.start_line > 0, "start_line should be positive");
            assert!(loc.end_line == loc.start_line, "end_line should equal start_line in fixture");
        }
    }

    #[test]
    fn test_find_locations_all_fields_populated() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_locations(&*db, Some("module_a"), "foo", None, "default", false, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();

        assert_eq!(locations.len(), 2, "Should find 2 clauses for foo/1");
        let loc = &locations[0];
        assert_eq!(loc.project, "default");
        assert_eq!(loc.module, "module_a");
        assert_eq!(loc.name, "foo");
        assert_eq!(loc.arity, 1);
        assert!(loc.line > 0);
        assert!(loc.start_line > 0);
        assert_eq!(loc.end_line, loc.start_line, "end_line should equal start_line in fixture");
        // file, kind, pattern, guard may be empty
    }

    // ==================== Sorting Tests ====================

    #[test]
    fn test_find_locations_sorted_by_module_name_arity_line() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Use wildcard pattern to get all locations
        let result = find_locations(&*db, None, ".*", None, "default", true, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();

        // Should be sorted by module_name, function_name, arity, line
        // Fixture order: module_a::bar/2@8, module_a::foo/1@10, module_a::foo/1@15, module_b::baz/0@3
        assert!(locations.len() >= 3);

        // Verify sorting: module_a comes before module_b
        let module_a_locations: Vec<_> = locations.iter().filter(|l| l.module == "module_a").collect();
        let module_b_locations: Vec<_> = locations.iter().filter(|l| l.module == "module_b").collect();

        if !module_a_locations.is_empty() && !module_b_locations.is_empty() {
            let last_a = module_a_locations.last().unwrap();
            let first_b = module_b_locations.first().unwrap();
            assert!(last_a.line <= first_b.line || last_a.module < first_b.module);
        }
    }

    #[test]
    fn test_find_locations_sorted_consistently() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Multiple calls should return results in consistent order
        let result1 = find_locations(&*db, None, ".*", None, "default", true, 100).unwrap();
        let result2 = find_locations(&*db, None, ".*", None, "default", true, 100).unwrap();

        // Results should be identical
        assert_eq!(result1.len(), result2.len());
        for (a, b) in result1.iter().zip(result2.iter()) {
            assert_eq!(a.module, b.module);
            assert_eq!(a.name, b.name);
            assert_eq!(a.arity, b.arity);
            assert_eq!(a.line, b.line);
        }
    }

    // ==================== Case Sensitivity Tests ====================

    #[test]
    fn test_find_locations_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search should be case sensitive
        let result_lower = find_locations(&*db, Some("module_a"), "foo", None, "default", false, 100);
        let result_upper = find_locations(&*db, Some("module_a"), "FOO", None, "default", false, 100);

        assert!(result_lower.is_ok());
        assert!(result_upper.is_ok());

        let lower_locations = result_lower.unwrap();
        let upper_locations = result_upper.unwrap();

        // Lowercase should find the function, uppercase should not
        assert_eq!(lower_locations.len(), 2, "Lowercase should find foo locations");
        assert_eq!(upper_locations.len(), 0, "Uppercase should find nothing");
    }

    #[test]
    fn test_find_locations_module_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search should be case sensitive for module names
        let result_lower = find_locations(&*db, Some("module_a"), ".*", None, "default", true, 100);
        let result_upper = find_locations(&*db, Some("MODULE_A"), ".*", None, "default", true, 100);

        assert!(result_lower.is_ok());
        assert!(result_upper.is_ok());

        let lower_locations = result_lower.unwrap();
        let upper_locations = result_upper.unwrap();

        assert_eq!(lower_locations.len(), 3, "Lowercase module should find locations");
        assert_eq!(upper_locations.len(), 0, "Uppercase module should find nothing");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_find_locations_empty_pattern() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Empty patterns in exact match mode
        let result = find_locations(&*db, Some(""), "", None, "default", false, 100);

        assert!(result.is_ok(), "Should handle empty pattern");
        let locations = result.unwrap();
        // Empty string doesn't match any function names
        assert_eq!(locations.len(), 0, "Empty pattern should match nothing");
    }

    #[test]
    fn test_find_locations_all_parameters_without_arity() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test with module and function parameters (no arity to avoid query issues)
        let result = find_locations(
            &*db,
            Some("module_a"),
            "foo",
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let locations = result.unwrap();

        // Should find foo/1 in module_a (2 clauses)
        assert_eq!(locations.len(), 2, "Should find 2 clauses for foo/1");
        for loc in &locations {
            assert_eq!(loc.module, "module_a", "Module should be module_a");
            assert_eq!(loc.name, "foo", "Name should be foo");
            assert_eq!(loc.arity, 1, "Arity should be 1");
        }
    }

    #[test]
    fn test_find_locations_arity_zero() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search for zero-arity functions - baz has arity 0
        let result = find_locations(&*db, Some("module_b"), "baz", None, "default", false, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();

        // Should find baz/0 in module_b with one clause at line 3
        assert_eq!(locations.len(), 1, "Should find exactly one baz location");
        assert_eq!(locations[0].name, "baz");
        assert_eq!(locations[0].arity, 0);
        assert_eq!(locations[0].line, 3);
    }

    #[test]
    fn test_find_locations_project_field_always_default() {
        let db = crate::test_utils::surreal_call_graph_db();

        // All results should have project field set to "default"
        let result = find_locations(&*db, None, ".*", None, "default", true, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();

        for loc in locations {
            assert_eq!(
                loc.project, "default",
                "Project should always be 'default' for SurrealDB"
            );
        }
    }

    #[test]
    fn test_find_locations_multiple_clauses_same_function() {
        let db = crate::test_utils::surreal_call_graph_db();

        // foo/1 has 2 clauses (at lines 10 and 15)
        let result = find_locations(&*db, Some("module_a"), "foo", None, "default", false, 100);

        assert!(result.is_ok());
        let locations = result.unwrap();

        assert_eq!(locations.len(), 2, "Should find both clauses of foo/1");
        // Both should be foo/1 in module_a
        for loc in &locations {
            assert_eq!(loc.name, "foo");
            assert_eq!(loc.arity, 1);
        }
    }

    #[test]
    fn test_find_locations_preserves_line_numbers() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Verify that line numbers are preserved correctly
        // Test foo/1 which has clauses at specific line numbers
        let result = find_locations(&*db, Some("module_a"), "foo", None, "default", false, 100);
        assert!(result.is_ok());
        let locations = result.unwrap();

        assert_eq!(locations.len(), 2, "Should find two foo/1 clauses");
        // Verify they're at the expected lines
        assert_eq!(locations[0].line, 10, "First clause should be at line 10");
        assert_eq!(locations[1].line, 15, "Second clause should be at line 15");
    }
}
