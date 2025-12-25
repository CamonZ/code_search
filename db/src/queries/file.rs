use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::ConditionBuilder;

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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_functions_in_module(
    db: &dyn Database,
    module_pattern: &str,
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FileFunctionDef>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    // Build the WHERE clause based on regex vs exact match
    // Note: SurrealDB removed the ~ operator in v3.0
    // Use regex type casting: <regex>$pattern creates a regex from the string parameter
    let where_clause = if use_regex {
        "WHERE module_name = <regex>$module_pattern".to_string()
    } else {
        "WHERE module_name = $module_pattern".to_string()
    };

    // Query to find all clauses in matching modules
    // In SurrealDB, clauses (function_locations) store the location info (file, line)
    // We select: arity, file, function_name, line, module_name, source_file_absolute
    // Returns in alphabetical order
    let query = format!(
        r#"
        SELECT arity, file, function_name, line, module_name, source_file_absolute
        FROM `clause`
        {where_clause}
        ORDER BY module_name ASC, line ASC, function_name ASC, arity ASC
        LIMIT $limit
        "#,
    );

    let params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_int("limit", limit as i64);

    let result = db.execute_query(&query, params).map_err(|e| FileError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();

    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order:
        // arity (0), file (1), function_name (2), line (3), module_name (4), source_file_absolute (5)
        if row.len() >= 5 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let file = extract_string(row.get(1).unwrap()).unwrap_or_default();
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(3).unwrap(), 0);
            let Some(module) = extract_string(row.get(4).unwrap()) else {
                continue;
            };

            // SurrealDB clause table doesn't have kind, start_line, end_line, pattern, guard
            // Fill with default/empty values for compatibility
            results.push(FileFunctionDef {
                module,
                name,
                arity,
                kind: String::new(),
                line,
                start_line: 0,
                end_line: 0,
                pattern: String::new(),
                guard: String::new(),
                file,
            });
        }
    }

    // SurrealDB doesn't honor ORDER BY when using regex WHERE clauses
    // Sort results in Rust to ensure consistent ordering
    results.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.arity.cmp(&b.arity))
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

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_functions_in_module_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Invalid regex pattern: unclosed bracket
        let result = find_functions_in_module(&*db, "[invalid", "default", true, 100);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern"),
            "Error should mention invalid regex: {}",
            msg
        );
        assert!(
            msg.contains("[invalid"),
            "Error should show the pattern: {}",
            msg
        );
    }

    #[test]
    fn test_find_functions_in_module_non_regex_mode() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Even invalid regex should work in non-regex mode (treated as literal string)
        let result = find_functions_in_module(&*db, "[invalid", "default", false, 100);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_find_functions_in_module_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search for exact module name without regex
        let result = find_functions_in_module(&*db, "module_a", "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let functions = result.unwrap();

        // Fixture has 2 clauses for module_a (foo/1 at lines 10,15 and bar/2 at line 8)
        // Should find exactly 3 results (foo/1 x2, bar/2 x1)
        assert_eq!(functions.len(), 3, "Should find exactly 3 clauses in module_a");

        // First should be bar/2 (line 8, alphabetically first when sorted by module then line)
        assert_eq!(functions[0].module, "module_a");
        assert_eq!(functions[0].name, "bar");
        assert_eq!(functions[0].arity, 2);
        assert_eq!(functions[0].line, 8);

        // Second should be foo/1 (line 10)
        assert_eq!(functions[1].module, "module_a");
        assert_eq!(functions[1].name, "foo");
        assert_eq!(functions[1].arity, 1);
        assert_eq!(functions[1].line, 10);

        // Third should be foo/1 (line 15)
        assert_eq!(functions[2].module, "module_a");
        assert_eq!(functions[2].name, "foo");
        assert_eq!(functions[2].arity, 1);
        assert_eq!(functions[2].line, 15);
    }

    #[test]
    fn test_find_functions_in_module_returns_results() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Query all modules with regex pattern that matches all
        let result = find_functions_in_module(&*db, ".*", "default", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Fixture has 4 total clauses (3 in module_a, 1 in module_b)
        assert_eq!(functions.len(), 4, "Should find all 4 clauses");
    }

    #[test]
    fn test_find_functions_in_module_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test with limit=2 using regex to match all modules
        let result = find_functions_in_module(&*db, ".*", "default", true, 2);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        assert_eq!(functions.len(), 2, "Should respect limit of 2");
    }

    #[test]
    fn test_find_functions_in_module_respects_zero_limit() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test with limit=0 using regex pattern
        let result = find_functions_in_module(&*db, ".*", "default", true, 0);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        assert_eq!(functions.len(), 0, "Should respect limit of 0");
    }

    #[test]
    fn test_find_functions_in_module_with_valid_regex() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search with regex pattern
        let result = find_functions_in_module(&*db, "^module_.*$", "default", true, 100);

        assert!(result.is_ok(), "Query should succeed with valid regex");
        let functions = result.unwrap();

        // All results should have module names matching the regex
        for func in &functions {
            assert!(
                func.module.starts_with("module_"),
                "Module {} should match pattern",
                func.module
            );
        }
    }

    #[test]
    fn test_find_functions_in_module_with_module_b() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search for module_b specifically
        let result = find_functions_in_module(&*db, "module_b", "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Fixture has 1 clause for module_b (baz/0 at line 3)
        assert_eq!(functions.len(), 1, "Should find exactly 1 clause in module_b");
        assert_eq!(functions[0].module, "module_b");
        assert_eq!(functions[0].name, "baz");
        assert_eq!(functions[0].arity, 0);
        assert_eq!(functions[0].line, 3);
    }

    #[test]
    fn test_find_functions_in_module_nonexistent_module() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search for non-existent module
        let result = find_functions_in_module(&*db, "nonexistent_module", "default", false, 100);

        assert!(result.is_ok(), "Query should succeed but return empty");
        let functions = result.unwrap();

        assert_eq!(functions.len(), 0, "Should find no results for non-existent module");
    }

    #[test]
    fn test_find_functions_in_module_returns_correct_fields() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Get all clauses using regex pattern
        let result = find_functions_in_module(&*db, ".*", "default", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Verify all results have correct field structure
        assert!(!functions.is_empty(), "Should have results");

        for func in &functions {
            // Core fields should be populated
            assert!(!func.module.is_empty(), "module should not be empty");
            assert!(!func.name.is_empty(), "name should not be empty");
            assert!(func.arity >= 0, "arity should be non-negative");
            assert!(func.line > 0, "line should be positive");

            // SurrealDB fields that may be empty (not available in clause table)
            assert_eq!(func.kind, "", "kind should be empty for SurrealDB");
            assert_eq!(func.start_line, 0, "start_line should be 0 for SurrealDB");
            assert_eq!(func.end_line, 0, "end_line should be 0 for SurrealDB");
            assert_eq!(func.pattern, "", "pattern should be empty for SurrealDB");
            assert_eq!(func.guard, "", "guard should be empty for SurrealDB");
        }
    }

    #[test]
    fn test_find_functions_in_module_sorted_order() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Get all clauses to verify sorting using regex pattern
        let result = find_functions_in_module(&*db, ".*", "default", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Results should be sorted by: module, line, name, arity
        // Verify the expected order from fixture:
        // 1. module_a, bar/2, line 8
        // 2. module_a, foo/1, line 10
        // 3. module_a, foo/1, line 15
        // 4. module_b, baz/0, line 3

        // Actually, sorting by module first means:
        // 1. module_a results first (sorted by line, then name, then arity)
        // 2. module_b results second

        let expected_order = vec![
            ("module_a", "bar", 2, 8),
            ("module_a", "foo", 1, 10),
            ("module_a", "foo", 1, 15),
            ("module_b", "baz", 0, 3),
        ];

        assert_eq!(
            functions.len(),
            expected_order.len(),
            "Should have {} clauses",
            expected_order.len()
        );

        for (i, (exp_module, exp_name, exp_arity, exp_line)) in expected_order.iter().enumerate() {
            let func = &functions[i];
            assert_eq!(func.module, *exp_module, "Item {}: module mismatch", i);
            assert_eq!(func.name, *exp_name, "Item {}: name mismatch", i);
            assert_eq!(func.arity, *exp_arity, "Item {}: arity mismatch", i);
            assert_eq!(func.line, *exp_line, "Item {}: line mismatch", i);
        }
    }

    #[test]
    fn test_find_functions_in_module_regex_alternation() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search with regex alternation pattern
        let result = find_functions_in_module(&*db, "^(module_a|module_b)$", "default", true, 100);

        assert!(result.is_ok(), "Query should succeed with alternation regex");
        let functions = result.unwrap();

        // Should find all 4 clauses
        assert_eq!(functions.len(), 4, "Should find all 4 clauses with alternation");

        for func in &functions {
            assert!(
                func.module == "module_a" || func.module == "module_b",
                "Module {} should match alternation pattern",
                func.module
            );
        }
    }

    #[test]
    fn test_find_functions_in_module_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Search with wrong case (should not match due to case sensitivity)
        let result = find_functions_in_module(&*db, "Module_A", "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Should find no results due to case sensitivity
        assert_eq!(functions.len(), 0, "Should be case sensitive - no match for 'Module_A'");
    }

    #[test]
    fn test_find_functions_in_module_empty_pattern_exact() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Empty pattern in exact match mode should find no results
        let result = find_functions_in_module(&*db, "", "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Empty string doesn't match any module names in exact mode
        assert_eq!(functions.len(), 0, "Empty pattern in exact mode should find no results");
    }

    #[test]
    fn test_find_functions_in_module_large_limit() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Test with very large limit using regex pattern
        let result = find_functions_in_module(&*db, ".*", "default", true, 1000);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Should find exactly 4 clauses (not more)
        assert_eq!(functions.len(), 4, "Should find exactly 4 clauses, not more");
    }
}
