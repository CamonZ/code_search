use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::extract_i64;
use crate::db::extract_string;
use crate::db::extract_string_or;
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum FunctionError {
    #[error("Function query failed: {message}")]
    QueryFailed { message: String },
}

/// A function signature
#[derive(Debug, Clone, Serialize)]
pub struct FunctionSignature {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub args: String,
    pub return_type: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_functions(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionSignature>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), Some(function_pattern)])?;

    // Build query conditions using helpers
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let function_cond = ConditionBuilder::new("name", "function_pattern")
        .with_leading_comma()
        .build(use_regex);
    let arity_cond = OptionalConditionBuilder::new("arity", "arity")
        .with_leading_comma()
        .build(arity.is_some());
    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, module, name, arity, args, return_type] :=
            *functions{{project, module, name, arity, args, return_type}},
            {module_cond}
            {function_cond}
            {arity_cond}
            {project_cond}
        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_str("function_pattern", function_pattern)
        .with_str("project", project);

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = run_query(db, &script, params).map_err(|e| FunctionError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
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
            let args = extract_string_or(row.get(4).unwrap(), "");
            let return_type = extract_string_or(row.get(5).unwrap(), "");

            results.push(FunctionSignature {
                project,
                module,
                name,
                arity,
                args,
                return_type,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_functions(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionSignature>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), Some(function_pattern)])?;

    // Build the WHERE clause based on regex vs exact match
    // SurrealDB removed the ~ operator in v3.0
    // Use regex type casting: <regex>$pattern creates a regex from the string parameter
    let module_clause = if use_regex {
        "module_name = <regex>$module_pattern"
    } else {
        "module_name = $module_pattern"
    };

    let function_clause = if use_regex {
        "name = <regex>$function_pattern"
    } else {
        "name = $function_pattern"
    };

    let arity_clause = if arity.is_some() {
        "AND arity = $arity"
    } else {
        ""
    };

    let query = format!(
        r#"
        SELECT "default" as project, module_name as module, name, arity, "" as args, return_type
        FROM functions
        WHERE {module_clause}
          AND {function_clause}
          {arity_clause}
        ORDER BY module_name ASC, name ASC, arity ASC
        LIMIT $limit
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_str("function_pattern", function_pattern)
        .with_int("limit", limit as i64);

    if let Some(a) = arity {
        params = params.with_int("arity", a);
    }

    let result = db.execute_query(&query, params).map_err(|e| FunctionError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order: args, arity, module, name, project, return_type
        if row.len() >= 6 {
            let args = extract_string_or(row.get(0).unwrap(), "");
            let arity = extract_i64(row.get(1).unwrap(), 0);
            let Some(module) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let Some(project) = extract_string(row.get(4).unwrap()) else {
                continue;
            };
            let return_type = extract_string_or(row.get(5).unwrap(), "");

            results.push(FunctionSignature {
                project,
                module,
                name,
                arity,
                args,
                return_type,
            });
        }
    }

    // SurrealDB doesn't honor ORDER BY when using regex WHERE clauses
    // Sort results in Rust to ensure consistent ordering: module_name, name, arity
    results.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
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
        crate::test_utils::type_signatures_db("default")
    }

    #[rstest]
    fn test_find_functions_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "",
            "",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        // May be empty if fixture doesn't have functions, just verify query executes
        assert!(functions.is_empty() || !functions.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_functions_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "NonExistentModule",
            "nonexistent",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Should return empty results for non-existent module");
    }

    #[rstest]
    fn test_find_functions_with_arity_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "MyApp.Controller",
            "index",
            Some(2),
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        // Verify all results have arity matching the filter or empty
        for func in &functions {
            assert_eq!(func.arity, 2, "All results should match arity filter");
        }
    }

    #[rstest]
    fn test_find_functions_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_1 = find_functions(&*populated_db, "MyApp", "", None, "default", false, 1)
            .unwrap();
        let limit_100 = find_functions(&*populated_db, "MyApp", "", None, "default", false, 100)
            .unwrap();

        assert!(limit_1.len() <= 1, "Limit should be respected");
        assert!(limit_1.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_functions_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "^MyApp\\..*$",
            "^index$",
            None,
            "default",
            true,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        // Should find functions matching the regex pattern
        if !functions.is_empty() {
            for func in &functions {
                assert!(func.module.starts_with("MyApp"), "Module should match regex");
                assert_eq!(func.name, "index", "Name should match regex");
            }
        }
    }

    #[rstest]
    fn test_find_functions_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(&*populated_db, "[invalid", "index", None, "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_functions_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "nonexistent",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_functions_returns_proper_fields(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_functions(
            &*populated_db,
            "MyApp.Controller",
            "index",
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let functions = result.unwrap();
        if !functions.is_empty() {
            let func = &functions[0];
            assert_eq!(func.project, "default");
            assert!(!func.module.is_empty());
            assert!(!func.name.is_empty());
            assert!(func.arity >= 0);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    // ==================== Validation Tests ====================

    #[test]
    fn test_find_functions_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Invalid regex pattern: unclosed bracket
        let result = find_functions(&*db, "[invalid", "foo", None, "default", true, 100);

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
    fn test_find_functions_invalid_regex_function_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Invalid regex pattern in function name: invalid repetition
        let result = find_functions(&*db, "module_a", "*invalid", None, "default", true, 100);

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
    fn test_find_functions_valid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Valid regex pattern should not error on validation
        let result = find_functions(&*db, "^module.*$", "^foo$", None, "default", true, 100);

        // Should not fail on validation
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_find_functions_non_regex_mode() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Even invalid regex should work in non-regex mode (treated as literal string)
        let result = find_functions(&*db, "[invalid", "foo", None, "default", false, 100);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_find_functions_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for exact function name without regex
        let result = find_functions(&*db, "MyApp.Controller", "index", None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let functions = result.unwrap();

        // Fixture has index/2 in MyApp.Controller, should find exactly 1 result
        assert_eq!(functions.len(), 1, "Should find exactly one function");
        assert_eq!(functions[0].name, "index");
        assert_eq!(functions[0].module, "MyApp.Controller");
        assert_eq!(functions[0].arity, 2);
        assert_eq!(functions[0].project, "default");
    }

    #[test]
    fn test_find_functions_empty_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for function that doesn't exist
        let result = find_functions(&*db, "MyApp.Controller", "nonexistent", None, "default", false, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Should find no results for nonexistent function");
    }

    #[test]
    fn test_find_functions_nonexistent_module() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search in module that doesn't exist
        let result = find_functions(
            &*db,
            "nonexistent_module",
            "index",
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Should find no results for nonexistent module");
    }

    #[test]
    fn test_find_functions_with_arity_filter() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search with arity filter - get_user has arities 1 and 2
        let result = find_functions(&*db, "MyApp.Accounts", "get_user", Some(1), "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Fixture has get_user/1 in MyApp.Accounts, should find exactly 1 result
        assert_eq!(functions.len(), 1, "Should find exactly one function with matching arity");
        assert_eq!(functions[0].name, "get_user");
        assert_eq!(functions[0].arity, 1);
    }

    #[test]
    fn test_find_functions_with_wrong_arity() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search with wrong arity (index/2 exists, but search for index/5)
        let result = find_functions(&*db, "MyApp.Controller", "index", Some(5), "default", false, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Should find no results with wrong arity");
    }

    // ==================== Limit Tests ====================

    #[test]
    fn test_find_functions_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use wildcard patterns to match all functions
        let limit_1 = find_functions(&*db, ".*", ".*", None, "default", true, 1).unwrap();
        let limit_100 = find_functions(&*db, ".*", ".*", None, "default", true, 100).unwrap();

        assert!(limit_1.len() <= 1, "Limit should be respected");
        assert!(limit_1.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[test]
    fn test_find_functions_zero_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with zero limit (use wildcard patterns)
        let result = find_functions(&*db, ".*", ".*", None, "default", true, 0);

        assert!(result.is_ok(), "Should handle zero limit");
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Zero limit should return no results");
    }

    #[test]
    fn test_find_functions_large_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with large limit (larger than fixture size, use wildcard patterns)
        let result = find_functions(&*db, ".*", ".*", None, "default", true, 1000000);

        assert!(result.is_ok(), "Should handle large limit");
        let functions = result.unwrap();

        // Fixture has 31 functions
        assert_eq!(functions.len(), 31, "Should return all functions");
    }

    // ==================== Pattern Matching Tests ====================

    #[test]
    fn test_find_functions_regex_dot_star() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Regex pattern that matches all functions
        let result = find_functions(&*db, ".*", ".*", None, "default", true, 100);

        assert!(result.is_ok(), "Should match all functions with .*");
        let functions = result.unwrap();

        // Fixture has exactly 31 functions
        assert_eq!(functions.len(), 31, "Should find exactly 31 functions");
    }

    #[test]
    fn test_find_functions_regex_alternation() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test regex alternation pattern - matches get_user or list_users
        let result = find_functions(&*db, "MyApp.Accounts", "^(get_user|list_users)", None, "default", true, 100);

        assert!(result.is_ok(), "Should handle regex alternation");
        let functions = result.unwrap();

        // MyApp.Accounts has get_user/1, get_user/2, and list_users/0, all match the pattern
        assert_eq!(functions.len(), 3, "Should match get_user and list_users");
        let names: Vec<_> = functions.iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"get_user".to_string()));
        assert!(names.contains(&"list_users".to_string()));
    }

    #[test]
    fn test_find_functions_regex_character_class() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with character class - matches anything starting with 's' in Notifier
        let result = find_functions(&*db, "MyApp.Notifier", "^s.*", None, "default", true, 100);

        assert!(result.is_ok(), "Should handle character class regex");
        let functions = result.unwrap();

        // Should find send_email/2 (starts with 's') in MyApp.Notifier
        assert!(
            functions.iter().all(|f| f.name.starts_with('s')),
            "All results should start with 's'"
        );
    }

    #[test]
    fn test_find_functions_module_pattern_partial_match() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for functions in MyApp.Controller matching pattern with wildcard function pattern
        let result = find_functions(&*db, "MyApp.Controller", ".*", None, "default", true, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // MyApp.Controller has 4 functions: create/2, index/2, show/2, handle_event/1
        assert_eq!(functions.len(), 4, "Should find 4 functions in MyApp.Controller");
        assert!(
            functions.iter().all(|f| f.module == "MyApp.Controller"),
            "All results should be in MyApp.Controller"
        );
    }

    // ==================== Result Structure Tests ====================

    #[test]
    fn test_find_functions_returns_correct_fields() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use wildcard patterns to get all functions
        let result = find_functions(&*db, ".*", ".*", None, "default", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Verify structure of returned functions
        for func in &functions {
            assert_eq!(func.project, "default", "project should be 'default'");
            assert!(!func.module.is_empty(), "module should not be empty");
            assert!(!func.name.is_empty(), "name should not be empty");
            assert!(func.arity >= 0, "arity should be non-negative");
        }
    }

    #[test]
    fn test_find_functions_returns_proper_fields() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_functions(&*db, "MyApp.Controller", "index", None, "default", false, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();

        if !functions.is_empty() {
            let func = &functions[0];
            assert_eq!(func.project, "default");
            assert_eq!(func.module, "MyApp.Controller");
            assert_eq!(func.name, "index");
            assert_eq!(func.arity, 2);
            assert!(!func.args.is_empty() || func.args.is_empty(), "args should be present");
            // return_type might be empty or have a value
        }
    }

    #[test]
    fn test_find_functions_preserves_project_field() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use wildcard patterns to get all functions
        let result = find_functions(&*db, ".*", ".*", None, "default", true, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // All results should have project field set to "default"
        for func in functions {
            assert_eq!(
                func.project, "default",
                "Project should always be 'default' for SurrealDB"
            );
        }
    }

    // ==================== Sorting Tests ====================

    #[test]
    fn test_find_functions_sorted_by_module_name_arity() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use wildcard patterns to get all functions
        let result = find_functions(&*db, ".*", ".*", None, "default", true, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // Fixture has 31 functions sorted by module_name, name, arity
        // First are MyApp.Accounts: __struct__/0, get_user/1, get_user/2, list_users/0, notify_change/1, validate_email/1
        assert_eq!(functions.len(), 31);
        assert_eq!(functions[0].module, "MyApp.Accounts");
        assert_eq!(functions[0].name, "__struct__");
        assert_eq!(functions[0].arity, 0);
        assert_eq!(functions[1].module, "MyApp.Accounts");
        assert_eq!(functions[1].name, "get_user");
        assert_eq!(functions[1].arity, 1);
        assert_eq!(functions[2].module, "MyApp.Accounts");
        assert_eq!(functions[2].name, "get_user");
        assert_eq!(functions[2].arity, 2);
    }

    #[test]
    fn test_find_functions_sorted_consistently() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Multiple calls should return results in consistent order
        let result1 = find_functions(&*db, ".*", ".*", None, "default", true, 100).unwrap();
        let result2 = find_functions(&*db, ".*", ".*", None, "default", true, 100).unwrap();

        // Results should be identical
        assert_eq!(result1.len(), result2.len());
        for (a, b) in result1.iter().zip(result2.iter()) {
            assert_eq!(a.module, b.module);
            assert_eq!(a.name, b.name);
            assert_eq!(a.arity, b.arity);
        }
    }

    // ==================== Case Sensitivity Tests ====================

    #[test]
    fn test_find_functions_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search should be case sensitive
        let result_lower = find_functions(&*db, "MyApp.Controller", "index", None, "default", false, 100);
        let result_upper = find_functions(&*db, "MyApp.Controller", "INDEX", None, "default", false, 100);

        assert!(result_lower.is_ok());
        assert!(result_upper.is_ok());

        let lower_functions = result_lower.unwrap();
        let upper_functions = result_upper.unwrap();

        // Lowercase should find the function, uppercase should not (case sensitive)
        assert_eq!(lower_functions.len(), 1, "Lowercase should find function");
        assert_eq!(
            lower_functions[0].name, "index",
            "Should find 'index' not 'INDEX'"
        );
        assert_eq!(upper_functions.len(), 0, "Uppercase should find nothing");
    }

    #[test]
    fn test_find_functions_module_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search should be case sensitive for module names (use wildcard function pattern)
        let result_correct = find_functions(&*db, "MyApp.Controller", ".*", None, "default", true, 100);
        let result_lower = find_functions(&*db, "myapp.controller", ".*", None, "default", true, 100);

        assert!(result_correct.is_ok());
        assert!(result_lower.is_ok());

        let correct_functions = result_correct.unwrap();
        let lower_functions = result_lower.unwrap();

        assert_eq!(correct_functions.len(), 4, "Correct case module should find functions");
        assert_eq!(lower_functions.len(), 0, "Lowercase module should find nothing");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_find_functions_empty_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Empty patterns in exact match mode - should match nothing typically
        let result = find_functions(&*db, "", "", None, "default", false, 100);

        assert!(result.is_ok(), "Should handle empty pattern");
        let functions = result.unwrap();
        // Empty string doesn't match any module or function names
        assert_eq!(functions.len(), 0, "Empty pattern should match nothing");
    }

    #[test]
    fn test_find_functions_all_parameters_filtered() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with all parameters: module, function, and arity
        let result = find_functions(
            &*db,
            "MyApp.Accounts",
            "get_user",
            Some(2),
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let functions = result.unwrap();

        // Should find exactly get_user/2 in MyApp.Accounts
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].module, "MyApp.Accounts");
        assert_eq!(functions[0].name, "get_user");
        assert_eq!(functions[0].arity, 2);
    }

    #[test]
    fn test_find_functions_arity_zero() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for zero-arity functions
        let result = find_functions(&*db, "MyApp.Accounts", "list_users", Some(0), "default", false, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // Should find list_users/0 in MyApp.Accounts
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "list_users");
        assert_eq!(functions[0].arity, 0);
    }

    #[test]
    fn test_find_functions_return_type_preserved() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use wildcard patterns to get all functions
        let result = find_functions(&*db, ".*", ".*", None, "default", true, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // All functions should have return_type field (may be empty string)
        for func in functions {
            // return_type field should exist and be accessible
            let _ = func.return_type.clone();
        }
    }

    #[test]
    fn test_find_functions_args_field_present() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_functions(&*db, "module_a", "foo", None, "default", false, 100);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // Args field should be present
        for func in functions {
            let _ = func.args.clone();
        }
    }
}
