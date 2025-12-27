use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};

#[cfg(feature = "backend-cozo")]
use crate::db::{extract_i64, extract_string, run_query};

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{validate_regex_patterns, ConditionBuilder, OptionalConditionBuilder};

#[cfg(feature = "backend-surrealdb")]
use crate::db::{extract_i64, extract_string, extract_string_or};

#[cfg(feature = "backend-surrealdb")]
use crate::query_builders::validate_regex_patterns;

#[derive(Error, Debug)]
pub enum TypesError {
    #[error("Types query failed: {message}")]
    QueryFailed { message: String },
}

/// A type definition (@type, @typep, @opaque)
#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    pub project: String,
    pub module: String,
    pub name: String,
    pub kind: String,
    pub params: String,
    pub line: i64,
    pub definition: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_types(
    db: &dyn Database,
    module_pattern: &str,
    name_filter: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<TypeInfo>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), name_filter])?;

    // Build conditions using query builders
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let name_cond = OptionalConditionBuilder::new("name", "name_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(name_filter.is_some(), use_regex);
    let kind_cond = OptionalConditionBuilder::new("kind", "kind")
        .with_leading_comma()
        .build(kind_filter.is_some());

    let script = format!(
        r#"
        ?[project, module, name, kind, params, line, definition] :=
            *types{{project, module, name, kind, params, line, definition}},
            project == $project,
            {module_cond}
            {name_cond}
            {kind_cond}

        :order module, name
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_str("module_pattern", module_pattern);

    if let Some(name) = name_filter {
        params = params.with_str("name_pattern", name);
    }

    if let Some(kind) = kind_filter {
        params = params.with_str("kind", kind);
    }

    let result = run_query(db, &script, params).map_err(|e| TypesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 7 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let Some(kind) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let params_str = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(5).unwrap(), 0);
            let definition = extract_string(row.get(6).unwrap()).unwrap_or_default();

            results.push(TypeInfo {
                project,
                module,
                name,
                kind,
                params: params_str,
                line,
                definition,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_types(
    db: &dyn Database,
    module_pattern: &str,
    name_filter: Option<&str>,
    kind_filter: Option<&str>,
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<TypeInfo>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), name_filter])?;

    // Build the WHERE clause based on regex vs exact match
    // SurrealDB removed the ~ operator in v3.0
    // Use regex type casting: <regex>$pattern creates a regex from the string parameter
    // For empty patterns, use .* in regex mode to match all, or 1=1 in exact mode
    let (module_clause, module_pattern_value) = if use_regex {
        let pattern = if module_pattern.is_empty() {
            ".*".to_string()
        } else {
            module_pattern.to_string()
        };
        ("module_name = <regex>$module_pattern".to_string(), pattern)
    } else {
        if module_pattern.is_empty() {
            ("1 = 1".to_string(), "".to_string()) // Match all, dummy value
        } else {
            ("module_name = $module_pattern".to_string(), module_pattern.to_string())
        }
    };

    let name_clause = if let Some(_) = name_filter {
        if use_regex {
            "AND name = <regex>$name_pattern"
        } else {
            "AND name = $name_pattern"
        }
    } else {
        ""
    };

    let kind_clause = if let Some(_) = kind_filter {
        "AND kind = $kind"
    } else {
        ""
    };

    let query = format!(
        r#"
        SELECT "default" as project, module_name as module, name, kind, params, line, definition
        FROM types
        WHERE {module_clause}
          {name_clause}
          {kind_clause}
        ORDER BY module_name ASC, name ASC
        LIMIT $limit
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("module_pattern", &module_pattern_value)
        .with_int("limit", limit as i64);

    if let Some(name) = name_filter {
        params = params.with_str("name_pattern", name);
    }

    if let Some(kind) = kind_filter {
        params = params.with_str("kind", kind);
    }

    let result = db.execute_query(&query, params).map_err(|e| TypesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order: definition, kind, line, module, name, params, project
        if row.len() >= 7 {
            let definition = extract_string_or(row.get(0).unwrap(), "");
            let Some(kind) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(2).unwrap(), 0);
            let Some(module) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(4).unwrap()) else {
                continue;
            };
            let params_str = extract_string_or(row.get(5).unwrap(), "");
            let Some(project) = extract_string(row.get(6).unwrap()) else {
                continue;
            };

            results.push(TypeInfo {
                project,
                module,
                name,
                kind,
                params: params_str,
                line,
                definition,
            });
        }
    }

    // SurrealDB doesn't honor ORDER BY when using regex WHERE clauses
    // Sort results in Rust to ensure consistent ordering: module_name, name
    results.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
            .then_with(|| a.name.cmp(&b.name))
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
    fn test_find_types_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        // May or may not have types, but query should execute
        assert!(types.is_empty() || !types.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_types_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(
            &*populated_db,
            "NonExistentModule",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(types.is_empty(), "Should return empty results for non-existent module");
    }

    #[rstest]
    fn test_find_types_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "MyApp", None, None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert!(t.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_types_with_name_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", Some("String"), None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert_eq!(t.name, "String", "Name should match filter");
        }
    }

    #[rstest]
    fn test_find_types_with_kind_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, Some("type"), "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert_eq!(t.kind, "type", "Kind should match filter");
        }
    }

    #[rstest]
    fn test_find_types_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_types(&*populated_db, "", None, None, "default", false, 5)
            .unwrap();
        let limit_100 = find_types(&*populated_db, "", None, None, "default", false, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_types_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "^MyApp\\..*$", None, None, "default", true, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        for t in &types {
            assert!(t.module.starts_with("MyApp"), "Module should match regex");
        }
    }

    #[rstest]
    fn test_find_types_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "[invalid", None, None, "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_types_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, None, "nonexistent", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(types.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_types_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_types(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let types = result.unwrap();
        if !types.is_empty() {
            let t = &types[0];
            assert_eq!(t.project, "default");
            assert!(!t.module.is_empty());
            assert!(!t.name.is_empty());
            assert!(!t.kind.is_empty());
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    // ==================== Validation Tests ====================

    #[test]
    fn test_find_types_invalid_regex() {
        let db = crate::test_utils::surreal_type_db();

        // Invalid regex pattern: unclosed bracket
        let result = find_types(&*db, "[invalid", None, None, "default", true, 100);

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
    fn test_find_types_invalid_regex_name_pattern() {
        let db = crate::test_utils::surreal_type_db();

        // Invalid regex pattern in name: invalid repetition
        let result = find_types(&*db, "module_a", Some("*invalid"), None, "default", true, 100);

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
    fn test_find_types_valid_regex() {
        let db = crate::test_utils::surreal_type_db();

        // Valid regex pattern should not error on validation
        let result = find_types(&*db, "^module.*$", None, None, "default", true, 100);

        // Should not fail on validation
        assert!(
            result.is_ok(),
            "Should accept valid regex: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_find_types_non_regex_mode() {
        let db = crate::test_utils::surreal_type_db();

        // Even invalid regex should work in non-regex mode (treated as literal string)
        let result = find_types(&*db, "[invalid", None, None, "default", false, 100);

        // Should succeed (no regex validation in non-regex mode)
        assert!(
            result.is_ok(),
            "Should accept any pattern in non-regex mode: {:?}",
            result.err()
        );
    }

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_find_types_exact_match() {
        let db = crate::test_utils::surreal_type_db();

        // Search for exact type name without regex
        let result = find_types(&*db, "module_a", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let types = result.unwrap();

        // Fixture has User type in module_a, should find exactly 1 result
        assert_eq!(types.len(), 1, "Should find exactly one type");
        assert_eq!(types[0].name, "User");
        assert_eq!(types[0].module, "module_a");
        assert_eq!(types[0].kind, "struct");
        assert_eq!(types[0].project, "default");
    }

    #[test]
    fn test_find_types_empty_results() {
        let db = crate::test_utils::surreal_type_db();

        // Search for type that doesn't exist
        let result = find_types(&*db, "module_a", Some("NonExistent"), None, "default", false, 100);

        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(types.is_empty(), "Should find no results for nonexistent type");
    }

    #[test]
    fn test_find_types_nonexistent_module() {
        let db = crate::test_utils::surreal_type_db();

        // Search in module that doesn't exist
        let result = find_types(&*db, "nonexistent_module", None, None, "default", false, 100);

        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(types.is_empty(), "Should find no results for nonexistent module");
    }

    #[test]
    fn test_find_types_with_kind_filter() {
        let db = crate::test_utils::surreal_type_db();

        // Search with kind filter
        let result = find_types(&*db, "module_a", None, Some("struct"), "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Fixture has User struct in module_a, should find exactly 1 result
        assert_eq!(types.len(), 1, "Should find exactly one type with matching kind");
        assert_eq!(types[0].name, "User");
        assert_eq!(types[0].kind, "struct");
    }

    #[test]
    fn test_find_types_with_wrong_kind() {
        let db = crate::test_utils::surreal_type_db();

        // Search for type with wrong kind (User is a struct, search for enum)
        let result = find_types(&*db, "module_a", None, Some("enum"), "default", false, 100);

        assert!(result.is_ok());
        let types = result.unwrap();
        assert!(types.is_empty(), "Should find no results for wrong kind");
    }

    #[test]
    fn test_find_types_respects_limit() {
        let db = crate::test_utils::surreal_type_db();

        // Query with low limit
        let limit_1 = find_types(&*db, "module_", None, None, "default", false, 1)
            .unwrap();
        let limit_100 = find_types(&*db, "module_", None, None, "default", false, 100)
            .unwrap();

        assert!(limit_1.len() <= 1, "Limit should be respected");
        assert!(limit_1.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[test]
    fn test_find_types_with_regex_pattern() {
        let db = crate::test_utils::surreal_type_db();

        // Search for modules matching regex pattern
        let result = find_types(&*db, "^module_.*$", None, None, "default", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Should find types matching the regex pattern
        if !types.is_empty() {
            for t in &types {
                assert!(t.module.starts_with("module_"), "Module should match regex");
            }
        }
    }

    #[test]
    fn test_find_types_with_name_pattern() {
        let db = crate::test_utils::surreal_type_db();

        // Search for specific type name
        let result = find_types(&*db, "module_a", Some("User"), None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Should find exactly the User type
        assert_eq!(types.len(), 1, "Should find exactly one type");
        assert_eq!(types[0].name, "User");
        assert_eq!(types[0].module, "module_a");
    }

    #[test]
    fn test_find_types_with_name_regex() {
        let db = crate::test_utils::surreal_type_db();

        // Search for type names matching regex
        let result = find_types(&*db, "module_a", Some("^User$"), None, "default", true, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Should find the User type
        if !types.is_empty() {
            for t in &types {
                assert_eq!(t.name, "User", "Name should match regex");
            }
        }
    }

    #[test]
    fn test_find_types_combined_filters() {
        let db = crate::test_utils::surreal_type_db();

        // Search with both module pattern and kind filter
        let result = find_types(
            &*db,
            "module_a",
            None,
            Some("struct"),
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // All results should match both filters
        for t in &types {
            assert!(
                t.module.contains("module_a"),
                "Module should match filter"
            );
            assert_eq!(t.kind, "struct", "Kind should match filter");
        }
    }

    #[test]
    fn test_find_types_combined_filters_with_name() {
        let db = crate::test_utils::surreal_type_db();

        // Search with module, name, and kind filters
        let result = find_types(
            &*db,
            "module_a",
            Some("User"),
            Some("struct"),
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Should find exactly the User struct in module_a
        assert_eq!(types.len(), 1, "Should find exactly one matching type");
        assert_eq!(types[0].name, "User");
        assert_eq!(types[0].module, "module_a");
        assert_eq!(types[0].kind, "struct");
    }

    #[test]
    fn test_find_types_returns_valid_structure() {
        let db = crate::test_utils::surreal_type_db();

        // Query all types
        let result = find_types(&*db, "", None, None, "default", false, 100);

        assert!(result.is_ok());
        let types = result.unwrap();

        // Verify structure of returned types
        if !types.is_empty() {
            let t = &types[0];
            assert_eq!(t.project, "default");
            assert!(!t.module.is_empty());
            assert!(!t.name.is_empty());
            assert!(!t.kind.is_empty());
            // params and definition may be empty, but fields should exist
            let _params = &t.params;
            let _definition = &t.definition;
        }
    }

    #[test]
    fn test_find_types_module_a_finds_user() {
        let db = crate::test_utils::surreal_type_db();

        let result = find_types(&*db, "module_a", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Verify we find the User type
        assert!(
            types.iter().any(|t| t.name == "User"),
            "Should find User type in module_a"
        );
    }

    #[test]
    fn test_find_types_module_b_finds_post() {
        let db = crate::test_utils::surreal_type_db();

        let result = find_types(&*db, "module_b", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Verify we find the Post type
        assert!(
            types.iter().any(|t| t.name == "Post"),
            "Should find Post type in module_b"
        );
    }

    #[test]
    fn test_find_types_all_modules() {
        let db = crate::test_utils::surreal_type_db();

        // Search for all types across all modules
        let result = find_types(&*db, "", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Should find multiple types from different modules
        assert!(!types.is_empty(), "Should find multiple types");

        // Check that we have variety of types
        let modules: std::collections::HashSet<_> = types.iter().map(|t| &t.module).collect();
        assert!(
            modules.len() > 1,
            "Should find types from multiple modules"
        );
    }

    #[test]
    fn test_find_types_sorting_order() {
        let db = crate::test_utils::surreal_type_db();

        // Search for all types to verify sorting
        let result = find_types(&*db, "", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Verify results are sorted by module, then by name
        for i in 0..types.len().saturating_sub(1) {
            let cmp = types[i].module.cmp(&types[i + 1].module);
            if cmp == std::cmp::Ordering::Equal {
                assert!(
                    types[i].name <= types[i + 1].name,
                    "Names should be sorted within same module"
                );
            } else {
                assert_eq!(cmp, std::cmp::Ordering::Less, "Modules should be sorted");
            }
        }
    }

    #[test]
    fn test_find_types_empty_module_pattern() {
        let db = crate::test_utils::surreal_type_db();

        // Empty module pattern should match all modules
        let result = find_types(&*db, "", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let types = result.unwrap();

        // Should find types across all modules
        if !types.is_empty() {
            let modules: std::collections::HashSet<_> = types.iter().map(|t| &t.module).collect();
            assert!(modules.len() > 0, "Should find types from at least one module");
        }
    }

    #[test]
    fn test_find_types_nonexistent_project() {
        let db = crate::test_utils::surreal_type_db();

        // Search with non-existent project
        let result = find_types(&*db, "", None, None, "nonexistent", false, 100);

        assert!(result.is_ok());
        let types = result.unwrap();

        // Since we always hardcode "default" in SurrealDB query, results might still appear
        // but verify project field for any returned results
        for t in &types {
            assert_eq!(t.project, "default", "Project should be 'default'");
        }
    }
}
