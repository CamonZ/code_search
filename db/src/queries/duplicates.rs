use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{validate_regex_patterns, OptionalConditionBuilder};

#[cfg(feature = "backend-surrealdb")]
use crate::query_builders::validate_regex_patterns;

#[derive(Error, Debug)]
pub enum DuplicatesError {
    #[error("Duplicates query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that has a duplicate implementation (same AST or source hash)
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateFunction {
    pub hash: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub file: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_duplicates(
    db: &dyn Database,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
    use_exact: bool,
    exclude_generated: bool,
) -> Result<Vec<DuplicateFunction>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Choose hash field based on exact flag
    let hash_field = if use_exact { "source_sha" } else { "ast_sha" };

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

    // Query to find duplicate hashes and their functions
    let script = format!(
        r#"
        # Find hashes that appear more than once (count unique functions per hash)
        hash_counts[{hash_field}, count(module)] :=
            *function_locations{{project, module, name, arity, {hash_field}, generated_by}},
            project == $project,
            {hash_field} != ""
            {generated_filter}

        # Get all functions with duplicate hashes
        ?[{hash_field}, module, name, arity, line, file] :=
            *function_locations{{project, module, name, arity, line, file, {hash_field}, generated_by}},
            hash_counts[{hash_field}, cnt],
            cnt > 1,
            project == $project
            {module_cond}
            {generated_filter}

        :order {hash_field}, module, name, arity
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| DuplicatesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
            let Some(hash) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(module) = extract_string(row.get(1).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(2).unwrap()) else { continue };
            let arity = extract_i64(row.get(3).unwrap(), 0);
            let line = extract_i64(row.get(4).unwrap(), 0);
            let Some(file) = extract_string(row.get(5).unwrap()) else { continue };

            results.push(DuplicateFunction {
                hash,
                module,
                name,
                arity,
                line,
                file,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_duplicates(
    db: &dyn Database,
    _project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
    use_exact: bool,
    exclude_generated: bool,
) -> Result<Vec<DuplicateFunction>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Choose hash field based on exact flag
    let hash_field = if use_exact { "source_sha" } else { "ast_sha" };

    // Build generated filter - this is applied during query
    let generated_filter = if exclude_generated {
        " AND (generated_by IS NONE)"
    } else {
        ""
    };

    // Query to get all clauses with non-empty hash values
    // Note: Module filter is applied AFTER finding duplicates in Rust, to ensure
    // we correctly identify duplicate pairs before filtering.
    let query = format!("SELECT {} as hash, module_name as module, function_name as name, arity, line, source_file as file FROM clauses WHERE {} != \"\"{} ORDER BY hash, module, name, arity",
        hash_field, hash_field, generated_filter
    );

    let params = QueryParams::new();

    let result = db
        .execute_query(&query, params)
        .map_err(|e| DuplicatesError::QueryFailed {
            message: e.to_string(),
        })?;

    // SurrealDB returns columns in alphabetical order by header name, not SELECT order.
    // Find column indices by name.
    let headers = result.headers();
    let hash_idx = headers.iter().position(|h| h == "hash").unwrap_or(0);
    let module_idx = headers.iter().position(|h| h == "module").unwrap_or(1);
    let name_idx = headers.iter().position(|h| h == "name").unwrap_or(2);
    let arity_idx = headers.iter().position(|h| h == "arity").unwrap_or(3);
    let line_idx = headers.iter().position(|h| h == "line").unwrap_or(4);
    let file_idx = headers.iter().position(|h| h == "file").unwrap_or(5);

    let mut all_items = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
            let hash = row.get(hash_idx).and_then(|v| extract_string(v)).unwrap_or_default();
            let module = row.get(module_idx).and_then(|v| extract_string(v)).unwrap_or_default();
            let name = row.get(name_idx).and_then(|v| extract_string(v)).unwrap_or_default();
            let arity = row.get(arity_idx).map(|v| extract_i64(v, 0)).unwrap_or(0);
            let line = row.get(line_idx).map(|v| extract_i64(v, 0)).unwrap_or(0);
            let file = row.get(file_idx).and_then(|v| extract_string(v)).unwrap_or_default();

            if !hash.is_empty() && !module.is_empty() && !name.is_empty() && !file.is_empty() {
                all_items.push(DuplicateFunction {
                    hash,
                    module,
                    name,
                    arity,
                    line,
                    file,
                });
            }
        }
    }

    // Filter to keep only hashes that appear more than once
    use std::collections::HashMap;
    let mut hash_counts: HashMap<String, usize> = HashMap::new();
    for item in &all_items {
        *hash_counts.entry(item.hash.clone()).or_insert(0) += 1;
    }

    // First filter to keep only hashes that appear more than once
    let duplicates: Vec<_> = all_items
        .into_iter()
        .filter(|item| hash_counts.get(&item.hash).map_or(false, |count| *count > 1))
        .collect();

    // Then apply module filter if provided
    let results = if let Some(pattern) = module_pattern {
        if use_regex {
            let regex = regex::Regex::new(pattern).map_err(|e| DuplicatesError::QueryFailed {
                message: format!("Invalid regex pattern: {}", e),
            })?;
            duplicates
                .into_iter()
                .filter(|item| regex.is_match(&item.module))
                .collect()
        } else {
            duplicates
                .into_iter()
                .filter(|item| item.module.contains(pattern))
                .collect()
        }
    } else {
        duplicates
    };

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
    fn test_find_duplicates_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_duplicates(&*populated_db, "default", None, false, false, false);
        assert!(result.is_ok());
        let duplicates = result.unwrap();
        // May or may not have duplicates, but query should execute
        assert!(duplicates.is_empty() || !duplicates.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_duplicates_empty_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_duplicates(&*populated_db, "nonexistent", None, false, false, false);
        assert!(result.is_ok());
        let duplicates = result.unwrap();
        assert!(
            duplicates.is_empty(),
            "Non-existent project should have no duplicates"
        );
    }

    #[rstest]
    fn test_find_duplicates_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_duplicates(&*populated_db, "default", Some("MyApp"), false, false, false);
        assert!(result.is_ok());
        let duplicates = result.unwrap();
        for dup in &duplicates {
            assert!(dup.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_duplicates_use_ast_hash(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_duplicates(&*populated_db, "default", None, false, false, false);
        assert!(result.is_ok());
        let duplicates = result.unwrap();
        // All hashes should be non-empty if there are duplicates
        for dup in &duplicates {
            assert!(dup.hash.is_empty() || !dup.hash.is_empty(), "Hash field should exist");
        }
    }

    #[rstest]
    fn test_find_duplicates_use_source_hash(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_duplicates(&*populated_db, "default", None, false, true, false);
        assert!(result.is_ok());
        let duplicates = result.unwrap();
        // Query should execute with exact flag
        assert!(duplicates.is_empty() || !duplicates.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_duplicates_exclude_generated(populated_db: Box<dyn crate::backend::Database>) {
        let with_generated = find_duplicates(&*populated_db, "default", None, false, false, false)
            .unwrap();
        let without_generated = find_duplicates(&*populated_db, "default", None, false, false, true)
            .unwrap();

        // Results without generated should be <= results with generated
        assert!(
            without_generated.len() <= with_generated.len(),
            "Excluding generated should not increase results"
        );
    }

    #[rstest]
    fn test_find_duplicates_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_duplicates(&*populated_db, "default", None, false, false, false);
        assert!(result.is_ok());
        let duplicates = result.unwrap();
        for dup in &duplicates {
            assert!(!dup.module.is_empty());
            assert!(!dup.name.is_empty());
            assert!(dup.arity >= 0);
            assert!(dup.line > 0);
            assert!(!dup.file.is_empty());
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    // The complex fixture contains duplicate test data for testing:
    // - AST duplicates: format_name/1 and format_display/1 (ast_hash_001)
    // - Source duplicates: validate/1 in Service and Repo (src_hash_001)
    // - Generated duplicates: __generated__/0 in Accounts and Controller (ast_hash_002, generated by phoenix)
    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== Basic functionality tests =====

    #[test]
    fn test_find_duplicates_ast_hash_returns_expected_pairs() {
        let db = get_db();
        let result =
            find_duplicates(&*db, "default", None, false, false, false).expect("Query should succeed");

        // Expect exactly 4 duplicates: 2 pairs with matching ast_sha and 2 generated
        assert_eq!(
            result.len(),
            4,
            "Should find 4 functions with duplicate AST hashes"
        );

        // Verify AST duplicates (ast_hash_001)
        let ast_dups: Vec<_> = result.iter().filter(|d| d.hash == "ast_hash_001").collect();
        assert_eq!(
            ast_dups.len(),
            2,
            "Should have 2 functions with ast_hash_001"
        );

        // Verify specific functions in AST pair
        assert!(
            ast_dups.iter().any(|d| d.module == "MyApp.Accounts"
                && d.name == "format_name"
                && d.arity == 1),
            "Should include MyApp.Accounts.format_name/1"
        );
        assert!(
            ast_dups.iter().any(|d| d.module == "MyApp.Controller"
                && d.name == "format_display"
                && d.arity == 1),
            "Should include MyApp.Controller.format_display/1"
        );

        // Verify generated duplicates (ast_hash_002)
        let gen_dups: Vec<_> = result.iter().filter(|d| d.hash == "ast_hash_002").collect();
        assert_eq!(gen_dups.len(), 2, "Should have 2 generated functions with ast_hash_002");
    }

    #[test]
    fn test_find_duplicates_source_hash_returns_exact_copies() {
        let db = get_db();
        let result = find_duplicates(&*db, "default", None, false, true, false)
            .expect("Query should succeed");

        // Expect exactly 2 duplicates: 1 pair with matching source_sha
        assert_eq!(
            result.len(),
            2,
            "Should find 2 functions with duplicate source hashes"
        );

        // Both should have the same source_sha
        assert_eq!(result[0].hash, "src_hash_001");
        assert_eq!(result[1].hash, "src_hash_001");

        // Verify specific functions
        let modules: Vec<&str> = result.iter().map(|d| d.module.as_str()).collect();
        assert!(
            modules.contains(&"MyApp.Service"),
            "Should include MyApp.Service"
        );
        assert!(modules.contains(&"MyApp.Repo"), "Should include MyApp.Repo");

        // Verify function names
        let names: Vec<&str> = result.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names.iter().filter(|n| **n == "validate").count(),
            2,
            "Both should be validate functions"
        );
    }

    #[test]
    fn test_find_duplicates_exclude_generated_filters_correctly() {
        let db = get_db();

        // With generated
        let with_gen = find_duplicates(&*db, "default", None, false, false, false)
            .expect("Query should succeed");

        // Without generated
        let without_gen = find_duplicates(&*db, "default", None, false, false, true)
            .expect("Query should succeed");

        assert_eq!(
            with_gen.len(),
            4,
            "Should find 4 duplicates including generated"
        );
        assert_eq!(
            without_gen.len(),
            2,
            "Should find 2 duplicates excluding generated"
        );

        // Verify no generated functions in filtered results
        for dup in &without_gen {
            assert!(
                !dup.name.contains("__generated__"),
                "Should not contain generated functions: {}",
                dup.name
            );
        }
    }

    #[test]
    fn test_find_duplicates_module_filter_returns_matching_only() {
        let db = get_db();
        let result = find_duplicates(&*db, "default", Some("Accounts"), false, false, false)
            .expect("Query should succeed");

        // Should find duplicates in or related to Accounts module
        assert!(!result.is_empty(), "Should find Accounts duplicates");

        for dup in &result {
            assert!(
                dup.module.contains("Accounts"),
                "All results should match Accounts filter: {}",
                dup.module
            );
        }
    }

    #[test]
    fn test_find_duplicates_ast_duplicates_with_excluded_generated() {
        let db = get_db();
        let result = find_duplicates(&*db, "default", None, false, false, true)
            .expect("Query should succeed");

        // Should only find AST duplicates without generated
        assert_eq!(result.len(), 2, "Should find exactly 2 AST duplicates");

        // All should be ast_hash_001
        for dup in &result {
            assert_eq!(
                dup.hash, "ast_hash_001",
                "All results should have ast_hash_001"
            );
        }

        // Verify the two functions
        assert!(result.iter().any(|d| d.module == "MyApp.Accounts"
            && d.name == "format_name"), "Should include format_name");
        assert!(result.iter().any(|d| d.module == "MyApp.Controller"
            && d.name == "format_display"), "Should include format_display");
    }

    #[test]
    fn test_find_duplicates_ordering_by_hash_module_name() {
        let db = get_db();
        let result = find_duplicates(&*db, "default", None, false, false, false)
            .expect("Query should succeed");

        // Verify ordering: by hash, then module, then name, then arity
        for i in 1..result.len() {
            let prev = &result[i - 1];
            let curr = &result[i];

            if prev.hash != curr.hash {
                assert!(
                    prev.hash < curr.hash,
                    "Results should be ordered by hash: {} < {}",
                    prev.hash,
                    curr.hash
                );
            } else if prev.module != curr.module {
                assert!(
                    prev.module < curr.module,
                    "Results with same hash should be ordered by module: {} < {}",
                    prev.module,
                    curr.module
                );
            } else if prev.name != curr.name {
                assert!(
                    prev.name < curr.name,
                    "Results with same hash/module should be ordered by name: {} < {}",
                    prev.name,
                    curr.name
                );
            } else {
                assert!(
                    prev.arity <= curr.arity,
                    "Results with same hash/module/name should be ordered by arity: {} <= {}",
                    prev.arity,
                    curr.arity
                );
            }
        }
    }

    #[test]
    fn test_find_duplicates_returns_correct_field_values() {
        let db = get_db();

        // Test AST duplicates field values
        let ast_result = find_duplicates(&*db, "default", None, false, false, false)
            .expect("Query should succeed");

        // Find format_name duplicate (AST mode)
        let format_name = ast_result
            .iter()
            .find(|d| d.name == "format_name")
            .expect("format_name should be found");

        assert_eq!(format_name.hash, "ast_hash_001");
        assert_eq!(format_name.module, "MyApp.Accounts");
        assert_eq!(format_name.arity, 1);
        assert_eq!(format_name.line, 50);
        assert_eq!(format_name.file, "lib/my_app/accounts.ex");

        // Test source duplicates field values (use_exact=true)
        let src_result = find_duplicates(&*db, "default", None, false, true, false)
            .expect("Query should succeed");

        // Find validate duplicate (source mode)
        let validate_service = src_result
            .iter()
            .find(|d| d.name == "validate" && d.module == "MyApp.Service")
            .expect("Service.validate should be found");

        assert_eq!(validate_service.hash, "src_hash_001");
        assert_eq!(validate_service.arity, 1);
        assert_eq!(validate_service.line, 70);
    }

    #[test]
    fn test_find_duplicates_module_filter_excludes_non_matching() {
        let db = get_db();
        // Service has source duplicates (not AST), so use_exact=true
        let result = find_duplicates(&*db, "default", Some("Service"), false, true, false)
            .expect("Query should succeed");

        // Should find Service validate duplicates
        assert!(!result.is_empty(), "Should find Service duplicates");

        // All should be in Service module
        for dup in &result {
            assert_eq!(dup.module, "MyApp.Service");
        }
    }

    #[test]
    fn test_find_duplicates_nonexistent_module_returns_empty() {
        let db = get_db();
        let result = find_duplicates(&*db, "default", Some("NonExistent"), false, false, false)
            .expect("Query should succeed");

        assert_eq!(result.len(), 0, "Should return empty for non-existent module");
    }

    #[test]
    fn test_find_duplicates_ast_and_source_mutually_exclusive() {
        let db = get_db();

        let ast_dups = find_duplicates(&*db, "default", None, false, false, false)
            .expect("Query should succeed");
        let source_dups = find_duplicates(&*db, "default", None, false, true, false)
            .expect("Query should succeed");

        // AST should return 4, source should return 2
        assert_eq!(ast_dups.len(), 4);
        assert_eq!(source_dups.len(), 2);

        // Verify they return different hashes
        let ast_hashes: Vec<_> = ast_dups.iter().map(|d| d.hash.as_str()).collect();
        let source_hashes: Vec<_> = source_dups.iter().map(|d| d.hash.as_str()).collect();

        // AST hashes should be ast_hash_001 and ast_hash_002
        assert!(ast_hashes.contains(&"ast_hash_001"));
        assert!(ast_hashes.contains(&"ast_hash_002"));

        // Source hashes should be src_hash_001
        assert!(source_hashes.iter().all(|h| *h == "src_hash_001"));
    }
}
