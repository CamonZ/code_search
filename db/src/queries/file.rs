use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};
use crate::query_builders::validate_regex_patterns;

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

pub fn find_functions_in_module(
    db: &dyn Database,
    module_pattern: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FileFunctionDef>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    // Build the WHERE clause based on regex vs exact match
    let where_clause = if use_regex {
        "WHERE string::matches(module_name, $module_pattern)".to_string()
    } else {
        "WHERE type::string(module_name) = $module_pattern".to_string()
    };

    // Query to find all clauses in matching modules
    // In SurrealDB, clauses (function_locations) store the location info
    // Select all fields needed for FileFunctionDef
    let query = format!(
        r#"
        SELECT
            arity,
            end_line,
            function_name,
            guard,
            kind,
            line,
            module_name,
            pattern,
            source_file,
            start_line
        FROM clauses
        {where_clause}
        ORDER BY module_name ASC, start_line ASC, function_name ASC, arity ASC, line ASC
        LIMIT $limit
        "#,
    );

    let params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_int("limit", limit as i64);

    let result = db
        .execute_query(&query, params)
        .map_err(|e| FileError::QueryFailed {
            message: e.to_string(),
        })?;

    let mut results = Vec::new();

    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order:
        // arity (0), end_line (1), function_name (2), guard (3), kind (4),
        // line (5), module_name (6), pattern (7), source_file (8), start_line (9)
        if row.len() >= 10 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let end_line = extract_i64(row.get(1).unwrap(), 0);
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let guard = extract_string(row.get(3).unwrap()).unwrap_or_default();
            let kind = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(5).unwrap(), 0);
            let Some(module) = extract_string(row.get(6).unwrap()) else {
                continue;
            };
            let pattern = extract_string(row.get(7).unwrap()).unwrap_or_default();
            let file = extract_string(row.get(8).unwrap()).unwrap_or_default();
            let start_line = extract_i64(row.get(9).unwrap(), 0);

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

    // SurrealDB doesn't honor ORDER BY when using regex WHERE clauses
    // Sort results in Rust to ensure consistent ordering: module, start_line, name, arity, line
    results.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
            .then_with(|| a.start_line.cmp(&b.start_line))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.arity.cmp(&b.arity))
            .then_with(|| a.line.cmp(&b.line))
    });

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_functions_in_module_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Invalid regex pattern: unclosed bracket
        let result = find_functions_in_module(&*db, "[invalid", true, 100);

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
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Even invalid regex should work in non-regex mode (treated as literal string)
        let result = find_functions_in_module(&*db, "[invalid", false, 100);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_find_functions_in_module_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for exact module name without regex
        let result = find_functions_in_module(&*db, "MyApp.Controller", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let functions = result.unwrap();

        // Controller has 10 clauses: index/2 (2), show/2 (2), create/2 (3), handle_event/1 (1), format_display/1 (1), __generated__/0 (1)
        assert_eq!(
            functions.len(),
            10,
            "Should find exactly 10 clauses in MyApp.Controller"
        );

        // First should be index/2 (line 5)
        assert_eq!(functions[0].module, "MyApp.Controller");
        assert_eq!(functions[0].name, "index");
        assert_eq!(functions[0].arity, 2);
        assert_eq!(functions[0].line, 5);

        // Second should be index/2 (line 7)
        assert_eq!(functions[1].module, "MyApp.Controller");
        assert_eq!(functions[1].name, "index");
        assert_eq!(functions[1].arity, 2);
        assert_eq!(functions[1].line, 7);
    }

    #[test]
    fn test_find_functions_in_module_returns_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Query all modules with regex pattern that matches all
        let result = find_functions_in_module(&*db, ".*", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Fixture has 44 total clauses across all 9 modules
        assert_eq!(functions.len(), 44, "Should find all 44 clauses");
    }

    #[test]
    fn test_find_functions_in_module_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with limit=2 using regex to match all modules
        let result = find_functions_in_module(&*db, ".*", true, 2);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        assert_eq!(functions.len(), 2, "Should respect limit of 2");
    }

    #[test]
    fn test_find_functions_in_module_respects_zero_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with limit=0 using regex pattern
        let result = find_functions_in_module(&*db, ".*", true, 0);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        assert_eq!(functions.len(), 0, "Should respect limit of 0");
    }

    #[test]
    fn test_find_functions_in_module_with_valid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search with regex pattern
        let result = find_functions_in_module(&*db, "^module_.*$", true, 100);

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
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for MyApp.Repo specifically
        let result = find_functions_in_module(&*db, "MyApp.Repo", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Fixture has 5 clauses for MyApp.Repo: get/2, all/1, insert/1, query/2, validate/1
        assert_eq!(
            functions.len(),
            5,
            "Should find exactly 5 clauses in MyApp.Repo"
        );
        assert_eq!(functions[0].module, "MyApp.Repo");
        assert_eq!(functions[0].name, "get");
        assert_eq!(functions[0].arity, 2);
        assert_eq!(functions[0].line, 10);
    }

    #[test]
    fn test_find_functions_in_module_nonexistent_module() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for non-existent module
        let result = find_functions_in_module(&*db, "nonexistent_module", false, 100);

        assert!(result.is_ok(), "Query should succeed but return empty");
        let functions = result.unwrap();

        assert_eq!(
            functions.len(),
            0,
            "Should find no results for non-existent module"
        );
    }

    #[test]
    fn test_find_functions_in_module_returns_correct_fields() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Get all clauses using regex pattern
        let result = find_functions_in_module(&*db, ".*", true, 100);

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

            // All clause fields should now be populated from the clauses table
            assert!(!func.kind.is_empty(), "kind should be populated");
            assert!(func.start_line > 0, "start_line should be positive");
            assert!(func.end_line >= func.start_line, "end_line should be >= start_line");
            // pattern and guard may be empty for some functions
        }
    }

    #[test]
    fn test_find_functions_in_module_sorted_order() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Get clauses for a specific module to verify sorting using regex pattern
        let result = find_functions_in_module(&*db, "MyApp.Accounts", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // MyApp.Accounts has 9 clauses sorted by start_line:
        // __struct__/0 at start_line 1
        // get_user/1 at start_lines 10, 12
        // get_user/2 at start_line 17
        // list_users/0 at start_line 24
        // validate_email/1 at start_line 30
        // notify_change/1 at start_line 40
        // format_name/1 at start_line 50
        // __generated__/0 at start_line 90
        assert_eq!(functions.len(), 9, "Should have 9 clauses");

        // Verify sorted by start_line
        assert_eq!(functions[0].start_line, 1); // __struct__
        assert_eq!(functions[1].start_line, 10);
        assert_eq!(functions[2].start_line, 12);
        assert_eq!(functions[3].start_line, 17);
        assert_eq!(functions[4].start_line, 24);
        assert_eq!(functions[5].start_line, 30);
        assert_eq!(functions[6].start_line, 40); // notify_change
        assert_eq!(functions[7].start_line, 50); // format_name
        assert_eq!(functions[8].start_line, 90); // __generated__
    }

    #[test]
    fn test_find_functions_in_module_regex_alternation() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search with regex alternation pattern for Controller and Accounts
        let result =
            find_functions_in_module(&*db, "MyApp\\.(Controller|Accounts)", true, 100);

        assert!(
            result.is_ok(),
            "Query should succeed with alternation regex"
        );
        let functions = result.unwrap();

        // Should find 19 clauses (10 from Controller + 9 from Accounts)
        assert_eq!(
            functions.len(),
            19,
            "Should find 19 clauses with alternation"
        );

        for func in &functions {
            assert!(
                func.module == "MyApp.Controller" || func.module == "MyApp.Accounts",
                "Module {} should match alternation pattern",
                func.module
            );
        }
    }

    #[test]
    fn test_find_functions_in_module_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search with wrong case (should not match due to case sensitivity)
        let result = find_functions_in_module(&*db, "myapp.controller", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Should find no results due to case sensitivity
        assert_eq!(
            functions.len(),
            0,
            "Should be case sensitive - no match for 'myapp.controller'"
        );
    }

    #[test]
    fn test_find_functions_in_module_empty_pattern_exact() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Empty pattern in exact match mode should find no results
        let result = find_functions_in_module(&*db, "", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Empty string doesn't match any module names in exact mode
        assert_eq!(
            functions.len(),
            0,
            "Empty pattern in exact mode should find no results"
        );
    }

    #[test]
    fn test_find_functions_in_module_large_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with very large limit using regex pattern
        let result = find_functions_in_module(&*db, ".*", true, 1000);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Should find exactly 44 clauses (not more)
        assert_eq!(
            functions.len(),
            44,
            "Should find exactly 44 clauses, not more"
        );
    }
}
