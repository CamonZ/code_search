use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, extract_string_or};
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::ConditionBuilder;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Search failed: {message}")]
    QueryFailed { message: String },
}

/// A module search result
#[derive(Debug, Clone, Serialize)]
pub struct ModuleResult {
    pub project: String,
    pub name: String,
    pub source: String,
}

/// A function search result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionResult {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub return_type: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn search_modules(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<ModuleResult>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern)])?;

    let match_cond = ConditionBuilder::new("name", "pattern").build(use_regex);
    let script = format!(
        r#"
        ?[project, name, source] := *modules{{project, name, source}},
            project = $project,
            {match_cond}
        :limit {limit}
        :order name
        "#,
    );

    let params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 3 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let source = extract_string_or(row.get(2).unwrap(), "");

            results.push(ModuleResult {
                project,
                name,
                source,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn search_modules(
    db: &dyn Database,
    pattern: &str,
    _project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<ModuleResult>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern)])?;

    // In SurrealDB, project is implicit (one DB per project)
    // Build the WHERE clause based on regex vs exact match
    // Note: SurrealDB removed the ~ operator in v3.0
    // Use regex type casting: <regex>$pattern creates a regex from the string parameter
    let where_clause = if use_regex {
        "WHERE name = <regex>$pattern".to_string()
    } else {
        "WHERE name = $pattern".to_string()
    };

    let query = format!(
        r#"
        SELECT "default" as project, name, source
        FROM modules
        {where_clause}
        ORDER BY name
        LIMIT $limit
        "#,
    );

    let params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_int("limit", limit as i64);

    let result = db.execute_query(&query, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order: name, project, source
        if row.len() >= 3 {
            let Some(name) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(project) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let source = extract_string_or(row.get(2).unwrap(), "");

            results.push(ModuleResult {
                project,
                name,
                source,
            });
        }
    }

    // SurrealDB doesn't honor ORDER BY when using regex WHERE clauses
    // Sort results in Rust to ensure consistent ordering
    results.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(results)
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn search_functions(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<FunctionResult>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern)])?;

    let match_cond = ConditionBuilder::new("name", "pattern").build(use_regex);
    let script = format!(
        r#"
        ?[project, module, name, arity, return_type] := *functions{{project, module, name, arity, return_type}},
            project = $project,
            {match_cond}
        :limit {limit}
        :order module, name, arity
        "#,
    );

    let params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 5 {
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
            let return_type = extract_string_or(row.get(4).unwrap(), "");

            results.push(FunctionResult {
                project,
                module,
                name,
                arity,
                return_type,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn search_functions(
    db: &dyn Database,
    pattern: &str,
    _project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<FunctionResult>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern)])?;

    // In SurrealDB, project is implicit (one DB per project)
    // Build the WHERE clause based on regex vs exact match
    // Note: SurrealDB removed the ~ operator in v3.0
    // Use regex type casting: <regex>$pattern creates a regex from the string parameter
    let where_clause = if use_regex {
        "WHERE name = <regex>$pattern".to_string()
    } else {
        "WHERE name = $pattern".to_string()
    };

    // Note: function table no longer has return_type field in SurrealDB schema
    // We return empty string for return_type to maintain API compatibility
    let query = format!(
        r#"
        SELECT "default" as project, module_name as module, name, arity
        FROM functions
        {where_clause}
        ORDER BY module_name ASC, name ASC, arity ASC
        LIMIT $limit
        "#,
    );

    let params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_int("limit", limit as i64);

    let result = db.execute_query(&query, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order: arity, module, name, project
        // Note: return_type is no longer in the schema, we return empty string
        if row.len() >= 4 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let Some(project) = extract_string(row.get(3).unwrap()) else {
                continue;
            };

            results.push(FunctionResult {
                project,
                module,
                name,
                arity,
                return_type: String::new(), // Not stored in SurrealDB schema
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

    #[test]
    fn test_search_modules_invalid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Invalid regex pattern: unclosed bracket
        let result = search_modules(&*db, "[invalid", "test_project", 10, true);

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
    fn test_search_functions_invalid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Invalid regex pattern: invalid repetition
        let result = search_functions(&*db, "*invalid", "test_project", 10, true);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern"),
            "Error should mention invalid regex: {}",
            msg
        );
        assert!(
            msg.contains("*invalid"),
            "Error should show the pattern: {}",
            msg
        );
    }

    #[test]
    fn test_search_modules_valid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Valid regex pattern should not error on validation (may or may not find results)
        let result = search_modules(&*db, "^test.*$", "test_project", 10, true);

        // Should not fail on validation (may return empty results, that's fine)
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_functions_valid_regex() {
        let db = crate::test_utils::call_graph_db("default");

        // Valid regex pattern should not error on validation
        let result = search_functions(&*db, "^get_.*$", "test_project", 10, true);

        // Should not fail on validation
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_modules_non_regex_mode() {
        let db = crate::test_utils::call_graph_db("default");

        // Even invalid regex should work in non-regex mode (treated as literal string)
        let result = search_modules(&*db, "[invalid", "test_project", 10, false);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_functions_non_regex_mode() {
        let db = crate::test_utils::call_graph_db("default");

        // Even invalid regex should work in non-regex mode
        let result = search_functions(&*db, "*invalid", "test_project", 10, false);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_search_modules_valid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Valid regex pattern should not error on validation (may or may not find results)
        let result = search_modules(&*db, "^module_.*$", "default", 10, true);

        // Should not fail on validation (may return empty results, that's fine)
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_modules_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Invalid regex pattern: unclosed bracket
        let result = search_modules(&*db, "[invalid", "default", 10, true);

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
    fn test_search_modules_non_regex_mode() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Even invalid regex should work in non-regex mode (treated as literal string)
        let result = search_modules(&*db, "[invalid", "default", 10, false);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_modules_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for exact module name without regex
        let result = search_modules(&*db, "MyApp.Accounts", "default", 10, false);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let modules = result.unwrap();

        // Fixture has MyApp.Accounts, so we should find exactly 1 result
        assert_eq!(modules.len(), 1, "Should find exactly one module");
        assert_eq!(modules[0].name, "MyApp.Accounts");
        assert_eq!(modules[0].project, "default");
        assert_eq!(modules[0].source, "unknown");
    }

    #[test]
    fn test_search_modules_with_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test limit parameter - fixture has 5 modules, limit to 1
        let result = search_modules(&*db, ".*", "default", 1, true);

        assert!(result.is_ok(), "Should respect limit parameter");
        let modules = result.unwrap();

        // Should return exactly 1 module (first one alphabetically: MyApp.Accounts)
        assert_eq!(modules.len(), 1, "Should respect limit of 1");
        assert_eq!(modules[0].name, "MyApp.Accounts");
    }

    #[test]
    fn test_search_functions_valid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Valid regex pattern should not error on validation
        let result = search_functions(&*db, "^foo.*$", "default", 10, true);

        // Should not fail on validation
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_functions_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Invalid regex pattern: invalid repetition
        let result = search_functions(&*db, "*invalid", "default", 10, true);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid regex pattern"),
            "Error should mention invalid regex: {}",
            msg
        );
        assert!(
            msg.contains("*invalid"),
            "Error should show the pattern: {}",
            msg
        );
    }

    #[test]
    fn test_search_functions_non_regex_mode() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Even invalid regex should work in non-regex mode
        let result = search_functions(&*db, "*invalid", "default", 10, false);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_search_functions_exact_match() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for exact function name without regex
        let result = search_functions(&*db, "index", "default", 10, false);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let functions = result.unwrap();

        // Fixture has index/2 in MyApp.Controller, should find exactly 1 result
        assert_eq!(functions.len(), 1, "Should find exactly one function");
        assert_eq!(functions[0].name, "index");
        assert_eq!(functions[0].module, "MyApp.Controller");
        assert_eq!(functions[0].arity, 2);
        assert_eq!(functions[0].project, "default");
        // Note: return_type is not stored in SurrealDB schema (removed for simplification)
    }

    #[test]
    fn test_search_functions_with_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test limit parameter - search for get_user which has 2 arities, limit to 1
        let result = search_functions(&*db, "get_user", "default", 1, false);

        assert!(result.is_ok(), "Should respect limit parameter");
        let functions = result.unwrap();

        // Should return exactly 1 function
        assert_eq!(functions.len(), 1, "Should respect limit of 1");
        assert_eq!(functions[0].name, "get_user");
        // Could be either arity 1 or 2 depending on database ordering
    }

    #[test]
    fn test_search_functions_returns_correct_fields() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Get all functions to verify field structure
        let result = search_functions(&*db, ".*", "default", 20, true);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Fixture has 15 functions, all should have correct fields
        assert_eq!(functions.len(), 15);
        for func in &functions {
            assert_eq!(func.project, "default");
            assert!(!func.module.is_empty(), "module should not be empty");
            assert!(!func.name.is_empty(), "name should not be empty");
            assert!(func.arity >= 0, "arity should be non-negative");
            // Note: return_type is not stored in SurrealDB schema (empty string)
        }
    }

    #[test]
    fn test_search_modules_returns_correct_fields() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Get all modules to verify field structure
        let result = search_modules(&*db, ".*", "default", 10, true);

        assert!(result.is_ok(), "Query should succeed");
        let modules = result.unwrap();

        // Fixture has 5 modules, all should have correct fields
        assert_eq!(modules.len(), 5);
        for module in &modules {
            assert_eq!(module.project, "default");
            assert!(!module.name.is_empty(), "name should not be empty");
            assert_eq!(module.source, "unknown");
        }
    }

    #[test]
    fn test_search_modules_with_special_regex_chars() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with more complex regex pattern
        let result = search_modules(&*db, "^mod.*_[ab]$", "default", 10, true);

        assert!(result.is_ok(), "Should handle complex regex: {:?}", result.err());
    }

    #[test]
    fn test_search_functions_with_special_regex_chars() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with more complex regex pattern for functions
        let result = search_functions(&*db, "^[a-z]+_.*", "default", 10, true);

        assert!(result.is_ok(), "Should handle complex regex: {:?}", result.err());
    }

    #[test]
    fn test_search_modules_no_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for pattern that doesn't match anything
        let result = search_modules(&*db, "xyz_nonexistent_12345", "default", 10, false);

        assert!(result.is_ok(), "Should return empty results instead of error");
        let modules = result.unwrap();

        // No modules match this pattern
        assert_eq!(modules.len(), 0, "Should find no matches");
    }

    #[test]
    fn test_search_functions_no_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for pattern that doesn't match anything
        let result = search_functions(&*db, "xyz_nonexistent_fn_12345", "default", 10, false);

        assert!(result.is_ok(), "Should return empty results instead of error");
        let functions = result.unwrap();

        // No functions match this pattern
        assert_eq!(functions.len(), 0, "Should find no matches");
    }

    #[test]
    fn test_search_modules_zero_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with zero limit (should return no results)
        let result = search_modules(&*db, ".*", "default", 0, true);

        assert!(result.is_ok(), "Should handle zero limit");
        let modules = result.unwrap();
        assert!(modules.is_empty(), "Zero limit should return no results");
    }

    #[test]
    fn test_search_functions_zero_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with zero limit (should return no results)
        let result = search_functions(&*db, ".*", "default", 0, true);

        assert!(result.is_ok(), "Should handle zero limit");
        let functions = result.unwrap();
        assert!(functions.is_empty(), "Zero limit should return no results");
    }

    #[test]
    fn test_search_modules_large_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with large limit (larger than result set)
        let result = search_modules(&*db, ".*", "default", 1000000, true);

        assert!(result.is_ok(), "Should handle large limit");
        let modules = result.unwrap();

        // Fixture has 5 modules, large limit should return all of them
        assert_eq!(modules.len(), 5, "Should return all 5 modules");
    }

    #[test]
    fn test_search_functions_large_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with large limit (larger than result set)
        let result = search_functions(&*db, ".*", "default", 1000000, true);

        assert!(result.is_ok(), "Should handle large limit");
        let functions = result.unwrap();

        // Fixture has 15 functions, large limit should return all of them
        assert_eq!(functions.len(), 15, "Should return all 15 functions");
    }

    #[test]
    fn test_search_modules_empty_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with empty pattern in exact match mode (no modules named "")
        let result = search_modules(&*db, "", "default", 10, false);

        assert!(result.is_ok(), "Should handle empty pattern");
        let modules = result.unwrap();
        // Empty string doesn't match any module names
        assert_eq!(modules.len(), 0, "Empty pattern should match nothing");
    }

    #[test]
    fn test_search_functions_empty_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with empty pattern in exact match mode (no functions named "")
        let result = search_functions(&*db, "", "default", 10, false);

        assert!(result.is_ok(), "Should handle empty pattern");
        let functions = result.unwrap();
        // Empty string doesn't match any function names
        assert_eq!(functions.len(), 0, "Empty pattern should match nothing");
    }

    #[test]
    fn test_search_modules_regex_dot_star() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with regex pattern that matches all modules
        let result = search_modules(&*db, ".*", "default", 10, true);

        assert!(result.is_ok(), "Should match all modules with .*");
        let modules = result.unwrap();

        // Fixture has exactly 5 modules (alphabetically sorted)
        assert_eq!(modules.len(), 5, "Should find exactly 5 modules");
        assert_eq!(modules[0].name, "MyApp.Accounts");
        assert_eq!(modules[1].name, "MyApp.Controller");
        assert_eq!(modules[2].name, "MyApp.Notifier");
        assert_eq!(modules[3].name, "MyApp.Repo");
        assert_eq!(modules[4].name, "MyApp.Service");
    }

    #[test]
    fn test_search_functions_regex_dot_star() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with regex pattern that matches all functions
        let result = search_functions(&*db, ".*", "default", 20, true);

        assert!(result.is_ok(), "Should match all functions with .*");
        let functions = result.unwrap();

        // Fixture has exactly 15 functions sorted by module_name, name, arity
        assert_eq!(functions.len(), 15, "Should find exactly 15 functions");
        // First function: MyApp.Accounts.get_user/1
        assert_eq!(functions[0].module, "MyApp.Accounts");
        assert_eq!(functions[0].name, "get_user");
        assert_eq!(functions[0].arity, 1);
        // Second function: MyApp.Accounts.get_user/2
        assert_eq!(functions[1].module, "MyApp.Accounts");
        assert_eq!(functions[1].name, "get_user");
        assert_eq!(functions[1].arity, 2);
    }

    #[test]
    fn test_search_modules_matches_specific_name() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for specific module that should exist
        let result = search_modules(&*db, "MyApp.Repo", "default", 10, false);

        assert!(result.is_ok(), "Should find MyApp.Repo without error");
        let modules = result.unwrap();

        // Must find exactly the module we're looking for
        assert_eq!(modules.len(), 1, "Should find exactly one module");
        assert_eq!(modules[0].name, "MyApp.Repo");
        assert_eq!(modules[0].project, "default");
    }

    #[test]
    fn test_search_functions_matches_specific_name() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search for specific function that should exist
        let result = search_functions(&*db, "send_email", "default", 10, false);

        assert!(result.is_ok(), "Should find send_email without error");
        let functions = result.unwrap();

        // Must find exactly the function we're looking for
        assert_eq!(functions.len(), 1, "Should find exactly one function");
        assert_eq!(functions[0].name, "send_email");
        assert_eq!(functions[0].module, "MyApp.Notifier");
        assert_eq!(functions[0].arity, 2);
    }

    #[test]
    fn test_search_modules_sorted_by_name() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Get all modules to verify sorting
        let result = search_modules(&*db, ".*", "default", 100, true);

        assert!(result.is_ok(), "Query should succeed");
        let modules = result.unwrap();

        // Fixture has 5 modules (alphabetically sorted)
        assert_eq!(modules.len(), 5);
        assert_eq!(modules[0].name, "MyApp.Accounts");
        assert_eq!(modules[1].name, "MyApp.Controller");
        assert_eq!(modules[2].name, "MyApp.Notifier");
        assert_eq!(modules[3].name, "MyApp.Repo");
        assert_eq!(modules[4].name, "MyApp.Service");
    }

    #[test]
    fn test_search_functions_sorted_by_module_name_arity() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Get all functions to verify sorting
        let result = search_functions(&*db, ".*", "default", 100, true);

        assert!(result.is_ok(), "Query should succeed");
        let functions = result.unwrap();

        // Fixture has 15 functions sorted by module_name, name, arity
        assert_eq!(functions.len(), 15);
        // First 4 are in MyApp.Accounts: get_user/1, get_user/2, list_users/0, validate_email/1
        assert_eq!(functions[0].module, "MyApp.Accounts");
        assert_eq!(functions[0].name, "get_user");
        assert_eq!(functions[0].arity, 1);
        assert_eq!(functions[1].module, "MyApp.Accounts");
        assert_eq!(functions[1].name, "get_user");
        assert_eq!(functions[1].arity, 2);
        assert_eq!(functions[2].module, "MyApp.Accounts");
        assert_eq!(functions[2].name, "list_users");
        assert_eq!(functions[2].arity, 0);
        assert_eq!(functions[3].module, "MyApp.Accounts");
        assert_eq!(functions[3].name, "validate_email");
        assert_eq!(functions[3].arity, 1);
    }

    #[test]
    fn test_search_modules_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search should be case sensitive
        let result_correct = search_modules(&*db, "MyApp.Accounts", "default", 10, false);
        let result_lower = search_modules(&*db, "myapp.accounts", "default", 10, false);

        assert!(result_correct.is_ok());
        assert!(result_lower.is_ok());

        let correct_modules = result_correct.unwrap();
        let lower_modules = result_lower.unwrap();

        // Correct case should find the module, lowercase should not (case sensitive)
        assert_eq!(correct_modules.len(), 1, "Correct case should find module");
        assert_eq!(correct_modules[0].name, "MyApp.Accounts");
        assert_eq!(lower_modules.len(), 0, "Lowercase should find nothing (case sensitive)");
    }

    #[test]
    fn test_search_functions_case_sensitive() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Search should be case sensitive
        let result_lower = search_functions(&*db, "get_user", "default", 10, false);
        let result_upper = search_functions(&*db, "GET_USER", "default", 10, false);

        assert!(result_lower.is_ok());
        assert!(result_upper.is_ok());

        let lower_functions = result_lower.unwrap();
        let upper_functions = result_upper.unwrap();

        // Lowercase should find the function (2 arities), uppercase should not (case sensitive)
        assert_eq!(lower_functions.len(), 2, "Lowercase should find functions");
        assert_eq!(lower_functions[0].name, "get_user");
        assert_eq!(upper_functions.len(), 0, "Uppercase should find nothing (case sensitive)");
    }

    #[test]
    fn test_search_modules_preserves_project_field() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Ensure project field is set correctly
        let result = search_modules(&*db, ".*", "default", 100, true);

        assert!(result.is_ok());
        let modules = result.unwrap();

        // All results should have project field populated
        for module in modules {
            assert_eq!(module.project, "default", "Project should always be 'default'");
        }
    }

    #[test]
    fn test_search_functions_preserves_project_field() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Ensure project field is set correctly
        let result = search_functions(&*db, ".*", "default", 100, true);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // All results should have project field populated
        for func in functions {
            assert_eq!(func.project, "default", "Project should always be 'default'");
        }
    }

    #[test]
    fn test_search_modules_arity_not_applicable() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Modules don't have arity, just verify structure is correct
        let result = search_modules(&*db, ".*", "default", 100, true);

        assert!(result.is_ok());
        let modules = result.unwrap();

        // Check structure of returned modules
        for module in modules {
            assert!(!module.name.is_empty(), "Module name should not be empty");
            assert!(!module.project.is_empty(), "Module project should not be empty");
        }
    }

    #[test]
    fn test_search_functions_arity_preserved() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Functions should preserve arity information
        let result = search_functions(&*db, ".*", "default", 100, true);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // Check structure of returned functions
        for func in functions {
            assert!(!func.name.is_empty(), "Function name should not be empty");
            assert!(!func.module.is_empty(), "Function module should not be empty");
            assert!(func.arity >= 0, "Function arity should be non-negative");
        }
    }

    #[test]
    fn test_search_modules_source_field_optional() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Source field should be optional
        let result = search_modules(&*db, ".*", "default", 100, true);

        assert!(result.is_ok());
        let modules = result.unwrap();

        // All modules should be returned even if source is empty
        // (the extract_string_or provides a default)
        for module in modules {
            assert!(!module.name.is_empty(), "Name should always be present");
            // source can be empty, that's OK
        }
    }

    #[test]
    fn test_search_functions_return_type_optional() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Return type should be optional
        let result = search_functions(&*db, ".*", "default", 100, true);

        assert!(result.is_ok());
        let functions = result.unwrap();

        // All functions should be returned even if return_type is empty
        // (the extract_string_or provides a default)
        for func in functions {
            assert!(!func.name.is_empty(), "Name should always be present");
            assert!(!func.module.is_empty(), "Module should always be present");
            // return_type can be empty, that's OK
        }
    }

    #[test]
    fn test_search_modules_with_digit_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with pattern containing digits
        let result = search_modules(&*db, ".*[0-9].*", "default", 10, true);

        assert!(result.is_ok(), "Should handle patterns with digits");
    }

    #[test]
    fn test_search_functions_with_digit_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with pattern containing digits
        let result = search_functions(&*db, ".*[0-9].*", "default", 10, true);

        assert!(result.is_ok(), "Should handle patterns with digits");
    }

    #[test]
    fn test_search_modules_with_underscore_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with pattern containing underscore
        let result = search_modules(&*db, "^[a-z]+_[a-z]$", "default", 10, true);

        assert!(result.is_ok(), "Should handle patterns with underscore");
    }

    #[test]
    fn test_search_functions_with_underscore_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with pattern containing underscore
        let result = search_functions(&*db, "^[a-z]+_[a-z]$", "default", 10, true);

        assert!(result.is_ok(), "Should handle patterns with underscore");
    }

    #[test]
    fn test_search_modules_whitespace_in_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with pattern containing whitespace (should find nothing typically)
        let result = search_modules(&*db, "mod ule", "default", 10, false);

        assert!(result.is_ok(), "Should handle patterns with whitespace");
    }

    #[test]
    fn test_search_functions_whitespace_in_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with pattern containing whitespace (should find nothing typically)
        let result = search_functions(&*db, "fun ction", "default", 10, false);

        assert!(result.is_ok(), "Should handle patterns with whitespace");
    }

    #[test]
    fn test_search_modules_single_char_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with single character pattern
        let result = search_modules(&*db, "a", "default", 10, false);

        assert!(result.is_ok(), "Should handle single character patterns");
    }

    #[test]
    fn test_search_functions_single_char_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test with single character pattern
        let result = search_functions(&*db, "o", "default", 10, false);

        assert!(result.is_ok(), "Should handle single character patterns");
    }

    #[test]
    fn test_search_modules_regex_alternation() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test regex alternation pattern - matches modules containing "Repo" or "Service"
        let result = search_modules(&*db, ".*(Repo|Service)$", "default", 10, true);

        assert!(result.is_ok(), "Should handle regex alternation");
        let modules = result.unwrap();

        // MyApp.Repo and MyApp.Service match this pattern
        assert_eq!(modules.len(), 2, "Should match two modules");
        assert_eq!(modules[0].name, "MyApp.Repo");
        assert_eq!(modules[1].name, "MyApp.Service");
    }

    #[test]
    fn test_search_functions_regex_alternation() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test regex alternation pattern - matches get, all, or insert functions
        let result = search_functions(&*db, "^(get|all|insert)", "default", 10, true);

        assert!(result.is_ok(), "Should handle regex alternation");
        let functions = result.unwrap();

        // get_user/1, get_user/2, get/2, all/1, insert/1 match this pattern (5 functions)
        assert_eq!(functions.len(), 5, "Should match 5 functions");
        // First two should be MyApp.Accounts.get_user/1 and /2
        assert_eq!(functions[0].name, "get_user");
        assert_eq!(functions[1].name, "get_user");
        // Then MyApp.Repo functions: all/1, get/2, insert/1
        assert_eq!(functions[2].name, "all");
        assert_eq!(functions[3].name, "get");
        assert_eq!(functions[4].name, "insert");
    }
}
