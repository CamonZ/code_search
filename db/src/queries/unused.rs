use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::OptionalConditionBuilder;

use crate::query_builders::validate_regex_patterns;

#[derive(Error, Debug)]
pub enum UnusedError {
    #[error("Unused query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that is never called
#[derive(Debug, Clone, Serialize)]
pub struct UnusedFunction {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub file: String,
    pub line: i64,
}

/// Generated function name patterns to exclude (Elixir compiler-generated)
const GENERATED_PATTERNS: &[&str] = &[
    "__struct__",
    "__using__",
    "__before_compile__",
    "__after_compile__",
    "__on_definition__",
    "__impl__",
    "__info__",
    "__protocol__",
    "__deriving__",
    "__changeset__",
    "__schema__",
    "__meta__",
    "__generated__",
];

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_unused_functions(
    db: &dyn Database,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    private_only: bool,
    public_only: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<UnusedFunction>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    // Build kind filter for private_only/public_only
    let kind_filter = if private_only {
        ", (kind == \"defp\" or kind == \"defmacrop\")".to_string()
    } else if public_only {
        ", (kind == \"def\" or kind == \"defmacro\")".to_string()
    } else {
        String::new()
    };

    // Find functions that exist in function_locations but are never called
    // We use function_locations as the source of "defined functions" and check
    // if they appear as a callee in the calls table
    let script = format!(
        r#"
        # All defined functions
        defined[module, name, arity, kind, file, start_line] :=
            *function_locations{{project, module, name, arity, kind, file, start_line}},
            project == $project
            {module_cond}
            {kind_filter}

        # All functions that are called (as callees)
        called[module, name, arity] :=
            *calls{{project, callee_module, callee_function, callee_arity}},
            project == $project,
            module = callee_module,
            name = callee_function,
            arity = callee_arity

        # Functions that are defined but never called
        ?[module, name, arity, kind, file, line] :=
            defined[module, name, arity, kind, file, line],
            not called[module, name, arity]

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new();
    params = params.with_str("project", project);
    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| UnusedError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(name) = extract_string(row.get(1).unwrap()) else { continue };
            let arity = extract_i64(row.get(2).unwrap(), 0);
            let Some(kind) = extract_string(row.get(3).unwrap()) else { continue };
            let Some(file) = extract_string(row.get(4).unwrap()) else { continue };
            let line = extract_i64(row.get(5).unwrap(), 0);

            // Filter out generated functions if requested
            if exclude_generated && GENERATED_PATTERNS.iter().any(|p| name.starts_with(p)) {
                continue;
            }

            results.push(UnusedFunction {
                module,
                name,
                arity,
                kind,
                file,
                line,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_unused_functions(
    db: &dyn Database,
    module_pattern: Option<&str>,
    _project: &str,
    use_regex: bool,
    private_only: bool,
    public_only: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<UnusedFunction>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Handle zero limit early
    if limit == 0 {
        return Ok(Vec::new());
    }

    // Build module filter clause using string::matches for regex
    let module_clause = match (module_pattern, use_regex) {
        (Some(_), true) => "AND string::matches(module_name, $module_pattern)",
        (Some(_), false) => "AND module_name = $module_pattern",
        (None, _) => "",
    };

    // Build kind filter for private_only/public_only
    let kind_clause = if private_only {
        r#"AND array::first(->has_clause->clauses.kind) IN ["defp", "defmacrop"]"#
    } else if public_only {
        r#"AND array::first(->has_clause->clauses.kind) IN ["def", "defmacro"]"#
    } else {
        ""
    };

    // Query functions that are NOT called (not in calls.out)
    // Use ->has_clause-> to get kind/file/line from clauses
    // array::first() for kind/file, math::min() for line (earliest clause)
    let query = format!(
        r#"
        SELECT
            module_name,
            name,
            arity,
            array::first(->has_clause->clauses.kind) as kind,
            array::first(->has_clause->clauses.source_file) as file,
            math::min(->has_clause->clauses.start_line) as line
        FROM functions
        WHERE id NOT IN (SELECT VALUE out FROM calls)
        {module_clause}
        {kind_clause}
        ORDER BY module_name, name, arity
        "#
    );

    let mut params = QueryParams::new();
    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = db.execute_query(&query, params).map_err(|e| UnusedError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns alphabetically (via BTreeMap):
        // 0: arity, 1: file, 2: kind, 3: line, 4: module_name, 5: name
        if row.len() >= 6 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let Some(file) = extract_string(row.get(1).unwrap()) else { continue; };
            let Some(kind) = extract_string(row.get(2).unwrap()) else { continue; };
            let line = extract_i64(row.get(3).unwrap(), 0);
            let Some(module) = extract_string(row.get(4).unwrap()) else { continue; };
            let Some(name) = extract_string(row.get(5).unwrap()) else { continue; };

            // Filter out generated functions if requested (done in Rust due to pattern list)
            if exclude_generated && GENERATED_PATTERNS.iter().any(|p| name.starts_with(p)) {
                continue;
            }

            results.push(UnusedFunction {
                module,
                name,
                arity,
                kind,
                file,
                line,
            });

            // Respect limit
            if results.len() >= limit as usize {
                break;
            }
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
    fn test_find_unused_functions_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            false,
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let unused = result.unwrap();
        // May or may not find unused functions depending on fixture data
        // Just verify the query executes successfully
        let _ = unused;
    }

    #[rstest]
    fn test_find_unused_functions_empty_module_filter(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            Some("NonExistentModule"),
            "default",
            false,
            false,
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let unused = result.unwrap();
        // Non-existent module filter should return empty
        assert!(unused.is_empty());
    }

    #[rstest]
    fn test_find_unused_functions_private_only_filter(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            true, // private_only
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let unused = result.unwrap();
        // If there are unused private functions, verify they are actually private
        for func in &unused {
            assert!(
                func.kind == "defp" || func.kind == "defmacrop",
                "Private filter should only return private functions, got {}",
                func.kind
            );
        }
    }

    #[rstest]
    fn test_find_unused_functions_public_only_filter(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            false,
            true, // public_only
            false,
            100,
        );
        assert!(result.is_ok());
        let unused = result.unwrap();
        // If there are unused public functions, verify they are actually public
        for func in &unused {
            assert!(
                func.kind == "def" || func.kind == "defmacro",
                "Public filter should only return public functions, got {}",
                func.kind
            );
        }
    }

    #[rstest]
    fn test_find_unused_functions_exclude_generated(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let with_generated = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            false,
            false,
            false, // include generated
            100,
        )
        .unwrap();

        let without_generated = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            false,
            false,
            true, // exclude generated
            100,
        )
        .unwrap();

        // Excluding generated should return same or fewer results
        assert!(without_generated.len() <= with_generated.len());

        // Verify no generated functions in excluded results
        for func in &without_generated {
            assert!(
                !func.name.starts_with("__"),
                "Excluded results should not contain generated functions"
            );
        }
    }

    #[rstest]
    fn test_find_unused_functions_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            false,
            false,
            false,
            5,
        )
        .unwrap();

        let limit_100 = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .unwrap();

        // Smaller limit should return fewer results
        assert!(limit_5.len() <= limit_100.len());
        assert!(limit_5.len() <= 5);
    }

    #[rstest]
    fn test_find_unused_functions_with_module_pattern(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            Some("MyApp.Accounts"),
            "default",
            false,
            false,
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let unused = result.unwrap();
        // All results should be from MyApp.Accounts module
        for func in &unused {
            assert_eq!(func.module, "MyApp.Accounts", "Module filter should match results");
        }
    }

    #[rstest]
    fn test_find_unused_functions_with_regex_pattern(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            Some("^MyApp\\.Accounts$"),
            "default",
            true, // use_regex
            false,
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let unused = result.unwrap();
        // All results should match the regex
        for func in &unused {
            assert_eq!(func.module, "MyApp.Accounts", "Regex pattern should match results");
        }
    }

    #[rstest]
    fn test_find_unused_functions_invalid_regex(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            Some("[invalid"),
            "default",
            true, // use_regex
            false,
            false,
            false,
            100,
        );
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_unused_functions_nonexistent_project(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            None,
            "nonexistent",
            false,
            false,
            false,
            false,
            100,
        );
        assert!(result.is_ok());
        let unused = result.unwrap();
        assert!(unused.is_empty(), "Nonexistent project should return no results");
    }

    #[rstest]
    fn test_find_unused_functions_result_fields_valid(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_unused_functions(
            &*populated_db,
            None,
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .unwrap();

        // Verify all result fields are populated
        for func in &result {
            assert!(!func.module.is_empty(), "Module should not be empty");
            assert!(!func.name.is_empty(), "Name should not be empty");
            assert!(func.arity >= 0, "Arity should be non-negative");
            assert!(!func.kind.is_empty(), "Kind should not be empty");
            assert!(!func.file.is_empty(), "File should not be empty");
            assert!(func.line > 0, "Line should be positive");
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    // The complex fixture contains:
    // - 9 modules: Controller, Accounts, Service, Repo, Notifier, Logger, Events, Cache, Metrics
    // - 37 functions total (31 original + 6 for duplicate testing)
    // - 24 calls (edges) including 3 cycles:
    //   - Cycle A (3 nodes): Service → Logger → Repo → Service
    //   - Cycle B (4 nodes): Controller → Events → Cache → Accounts → Controller
    //   - Cycle C (5 nodes): Notifier → Metrics → Logger → Events → Cache → Notifier
    //
    // Unused functions (16 total - 10 original + 6 new for duplicate testing):
    // 1. MyApp.Accounts.__generated__/0 - def - line 90 (generated, duplicate)
    // 2. MyApp.Accounts.__struct__/0 - def - line 1 (generated)
    // 3. MyApp.Accounts.format_name/1 - def - line 50 (duplicate)
    // 4. MyApp.Accounts.validate_email/1 - defp - line 30
    // 5. MyApp.Cache.fetch/1 - def - line 16
    // 6. MyApp.Controller.__generated__/0 - def - line 100 (generated, duplicate)
    // 7. MyApp.Controller.create/2 - def - line 20
    // 8. MyApp.Controller.format_display/1 - def - line 60 (duplicate)
    // 9. MyApp.Controller.index/2 - def - line 5
    // 10. MyApp.Controller.show/2 - def - line 12
    // 11. MyApp.Events.subscribe/2 - def - line 18
    // 12. MyApp.Logger.debug/1 - defp - line 18
    // 13. MyApp.Metrics.increment/1 - def - line 12
    // 14. MyApp.Repo.validate/1 - def - line 80 (duplicate)
    // 15. MyApp.Service.transform_data/1 - defp - line 22
    // 16. MyApp.Service.validate/1 - def - line 70 (duplicate)
    //
    // Private unused (3): validate_email, debug, transform_data
    // Public unused (13): __struct__, __generated__ x2, format_name, format_display, fetch, create, index, show, subscribe, increment, validate x2
    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== Basic functionality tests =====

    #[test]
    fn test_find_unused_functions_returns_exactly_16() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        // Exactly 16 unused functions in fixture (10 original + 6 for duplicates)
        assert_eq!(
            unused.len(),
            16,
            "Should find exactly 16 unused functions, got {}: {:?}",
            unused.len(),
            unused.iter().map(|f| format!("{}.{}/{}", f.module, f.name, f.arity)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_find_unused_functions_contains_expected_functions() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        // Build a set of expected unused function signatures (16 total)
        let expected = vec![
            ("MyApp.Accounts", "__generated__", 0),  // new for duplicates
            ("MyApp.Accounts", "__struct__", 0),
            ("MyApp.Accounts", "format_name", 1),    // new for duplicates
            ("MyApp.Accounts", "validate_email", 1),
            ("MyApp.Cache", "fetch", 1),
            ("MyApp.Controller", "__generated__", 0), // new for duplicates
            ("MyApp.Controller", "create", 2),
            ("MyApp.Controller", "format_display", 1), // new for duplicates
            ("MyApp.Controller", "index", 2),
            ("MyApp.Controller", "show", 2),
            ("MyApp.Events", "subscribe", 2),
            ("MyApp.Logger", "debug", 1),
            ("MyApp.Metrics", "increment", 1),
            ("MyApp.Repo", "validate", 1),           // new for duplicates
            ("MyApp.Service", "transform_data", 1),
            ("MyApp.Service", "validate", 1),        // new for duplicates
        ];

        for (module, name, arity) in &expected {
            let found = unused.iter().any(|f| {
                f.module == *module && f.name == *name && f.arity == *arity as i64
            });
            assert!(
                found,
                "Expected unused function {}.{}/{} not found in results",
                module, name, arity
            );
        }
    }

    #[test]
    fn test_find_unused_functions_first_result_is_accounts_generated() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        // Ordered by module, name, arity - first should be MyApp.Accounts.__generated__/0
        // (__generated__ comes before __struct__ alphabetically)
        assert!(!unused.is_empty(), "Should have results");
        let first = &unused[0];
        assert_eq!(first.module, "MyApp.Accounts");
        assert_eq!(first.name, "__generated__");
        assert_eq!(first.arity, 0);
        assert_eq!(first.kind, "def");
        assert_eq!(first.file, "lib/my_app/accounts.ex");
        assert_eq!(first.line, 90);
    }

    #[test]
    fn test_find_unused_functions_validate_email_details() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        let validate_email = unused.iter().find(|f| f.name == "validate_email");
        assert!(validate_email.is_some(), "Should find validate_email");

        let func = validate_email.unwrap();
        assert_eq!(func.module, "MyApp.Accounts");
        assert_eq!(func.name, "validate_email");
        assert_eq!(func.arity, 1);
        assert_eq!(func.kind, "defp");
        assert_eq!(func.file, "lib/my_app/accounts.ex");
        assert_eq!(func.line, 30);
    }

    #[test]
    fn test_find_unused_functions_transform_data_details() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        let transform_data = unused.iter().find(|f| f.name == "transform_data");
        assert!(transform_data.is_some(), "Should find transform_data");

        let func = transform_data.unwrap();
        assert_eq!(func.module, "MyApp.Service");
        assert_eq!(func.name, "transform_data");
        assert_eq!(func.arity, 1);
        assert_eq!(func.kind, "defp");
        assert_eq!(func.file, "lib/my_app/service.ex");
        assert_eq!(func.line, 22);
    }

    #[test]
    fn test_find_unused_functions_controller_index_details() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        let index = unused.iter().find(|f| f.name == "index");
        assert!(index.is_some(), "Should find index");

        let func = index.unwrap();
        assert_eq!(func.module, "MyApp.Controller");
        assert_eq!(func.name, "index");
        assert_eq!(func.arity, 2);
        assert_eq!(func.kind, "def");
        assert_eq!(func.file, "lib/my_app/controller.ex");
        assert_eq!(func.line, 5);
    }

    // ===== Visibility filtering tests =====

    #[test]
    fn test_find_unused_functions_private_only_returns_exactly_3() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, true, false, false, 100)
            .expect("Query should succeed");

        // Exactly 3 unused private functions: validate_email/1, debug/1, transform_data/1
        assert_eq!(
            unused.len(),
            3,
            "Should find exactly 3 unused private functions, got {}: {:?}",
            unused.len(),
            unused.iter().map(|f| format!("{}.{}/{}", f.module, f.name, f.arity)).collect::<Vec<_>>()
        );

        // Verify they are the expected functions
        let names: std::collections::HashSet<_> = unused.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("validate_email"), "Should contain validate_email");
        assert!(names.contains("debug"), "Should contain debug");
        assert!(names.contains("transform_data"), "Should contain transform_data");

        // All should be private
        for func in &unused {
            assert!(
                func.kind == "defp" || func.kind == "defmacrop",
                "Private filter should only return private functions, got {} for {}",
                func.kind,
                func.name
            );
        }
    }

    #[test]
    fn test_find_unused_functions_public_only_returns_exactly_13() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, true, false, 100)
            .expect("Query should succeed");

        // Exactly 13 unused public functions (16 total - 3 private: validate_email, debug, transform_data)
        assert_eq!(
            unused.len(),
            13,
            "Should find exactly 13 unused public functions, got {}: {:?}",
            unused.len(),
            unused.iter().map(|f| format!("{}.{}/{}", f.module, f.name, f.arity)).collect::<Vec<_>>()
        );

        // All should be public
        for func in &unused {
            assert!(
                func.kind == "def" || func.kind == "defmacro",
                "Public filter should only return public functions, got {} for {}",
                func.kind,
                func.name
            );
        }
    }

    #[test]
    fn test_find_unused_functions_private_only_validate_email() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, true, false, false, 100)
            .expect("Query should succeed");

        let validate_email = unused.iter().find(|f| f.name == "validate_email");
        assert!(validate_email.is_some(), "Should find validate_email in private results");

        let func = validate_email.unwrap();
        assert_eq!(func.module, "MyApp.Accounts");
        assert_eq!(func.kind, "defp");
    }

    #[test]
    fn test_find_unused_functions_private_only_transform_data() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, true, false, false, 100)
            .expect("Query should succeed");

        let transform_data = unused.iter().find(|f| f.name == "transform_data");
        assert!(transform_data.is_some(), "Should find transform_data in private results");

        let func = transform_data.unwrap();
        assert_eq!(func.module, "MyApp.Service");
        assert_eq!(func.kind, "defp");
    }

    #[test]
    fn test_find_unused_functions_private_and_public_sum_to_total() {
        let db = get_db();
        let private = find_unused_functions(&*db, None, "default", false, true, false, false, 100)
            .expect("Query should succeed");
        let public = find_unused_functions(&*db, None, "default", false, false, true, false, 100)
            .expect("Query should succeed");

        // Private (3) + Public (13) = Total (16)
        assert_eq!(
            private.len() + public.len(),
            16,
            "Private ({}) + Public ({}) should equal total unused (16)",
            private.len(),
            public.len()
        );
    }

    // ===== Generated function filtering tests =====

    #[test]
    fn test_find_unused_functions_exclude_generated_returns_exactly_13() {
        let db = get_db();
        let without_generated = find_unused_functions(&*db, None, "default", false, false, false, true, 100)
            .expect("Query should succeed");

        // 16 total unused - 3 generated (__struct__, __generated__ x2) = 13
        assert_eq!(
            without_generated.len(),
            13,
            "Should find exactly 13 non-generated unused functions, got {}: {:?}",
            without_generated.len(),
            without_generated.iter().map(|f| format!("{}.{}/{}", f.module, f.name, f.arity)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_find_unused_functions_exclude_generated_removes_struct() {
        let db = get_db();
        let with_generated = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");
        let without_generated = find_unused_functions(&*db, None, "default", false, false, false, true, 100)
            .expect("Query should succeed");

        // With generated should have __struct__ and __generated__, without should not
        let has_struct_with = with_generated.iter().any(|f| f.name == "__struct__");
        let has_struct_without = without_generated.iter().any(|f| f.name == "__struct__");
        let has_generated_with = with_generated.iter().any(|f| f.name == "__generated__");
        let has_generated_without = without_generated.iter().any(|f| f.name == "__generated__");

        assert!(has_struct_with, "__struct__ should be in unfiltered results");
        assert!(!has_struct_without, "__struct__ should NOT be in filtered results");
        assert!(has_generated_with, "__generated__ should be in unfiltered results");
        assert!(!has_generated_without, "__generated__ should NOT be in filtered results");

        // Difference should be exactly 3 (1 __struct__ + 2 __generated__)
        assert_eq!(
            with_generated.len() - without_generated.len(),
            3,
            "Excluding generated should remove exactly 3 functions"
        );
    }

    #[test]
    fn test_find_unused_functions_exclude_generated_no_dunder_names() {
        let db = get_db();
        let without_generated = find_unused_functions(&*db, None, "default", false, false, false, true, 100)
            .expect("Query should succeed");

        for func in &without_generated {
            assert!(
                !func.name.starts_with("__"),
                "Excluded results should not contain __ prefix, found: {}",
                func.name
            );
        }
    }

    // ===== Module pattern filtering tests =====

    #[test]
    fn test_find_unused_functions_controller_module_returns_exactly_3() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("MyApp.Controller"),
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        // Controller has 5 unused functions: __generated__, create, format_display, index, show
        assert_eq!(
            unused.len(),
            5,
            "Should find exactly 5 unused Controller functions, got {}: {:?}",
            unused.len(),
            unused.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
        );

        let names: std::collections::HashSet<_> = unused.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("__generated__"), "Should contain __generated__");
        assert!(names.contains("create"), "Should contain create");
        assert!(names.contains("format_display"), "Should contain format_display");
        assert!(names.contains("index"), "Should contain index");
        assert!(names.contains("show"), "Should contain show");
    }

    #[test]
    fn test_find_unused_functions_accounts_module_returns_exactly_4() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("MyApp.Accounts"),
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        // Accounts has 4 unused functions: __generated__, __struct__, format_name, validate_email
        assert_eq!(
            unused.len(),
            4,
            "Should find exactly 4 unused Accounts functions, got {}: {:?}",
            unused.len(),
            unused.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
        );

        let names: std::collections::HashSet<_> = unused.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("__generated__"), "Should contain __generated__");
        assert!(names.contains("__struct__"), "Should contain __struct__");
        assert!(names.contains("format_name"), "Should contain format_name");
        assert!(names.contains("validate_email"), "Should contain validate_email");
    }

    #[test]
    fn test_find_unused_functions_repo_module_returns_1() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("MyApp.Repo"),
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        // Repo has 1 unused function: validate (added for duplicate testing)
        assert_eq!(
            unused.len(),
            1,
            "Should find 1 unused Repo function (validate), got {}: {:?}",
            unused.len(),
            unused.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(unused[0].name, "validate");
    }

    #[test]
    fn test_find_unused_functions_service_module_returns_exactly_2() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("MyApp.Service"),
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        // Service has 2 unused functions: transform_data, validate
        assert_eq!(unused.len(), 2, "Should find exactly 2 unused Service functions");
        let names: std::collections::HashSet<_> = unused.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("transform_data"), "Should contain transform_data");
        assert!(names.contains("validate"), "Should contain validate");
    }

    #[test]
    fn test_find_unused_functions_notifier_module_returns_0() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("MyApp.Notifier"),
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        // Notifier has 0 unused functions (both send_email and format_message are called)
        assert!(
            unused.is_empty(),
            "Should find no unused Notifier functions, got {}: {:?}",
            unused.len(),
            unused.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_find_unused_functions_with_nonexistent_module() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("NonExistentModule"),
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(unused.is_empty(), "Should return empty for non-existent module");
    }

    #[test]
    fn test_find_unused_functions_with_regex_controller_pattern() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("^MyApp\\.Controller$"),
            "default",
            true,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        // Same as exact match - 5 functions
        assert_eq!(unused.len(), 5, "Regex should match Controller exactly");
        for func in &unused {
            assert_eq!(func.module, "MyApp.Controller");
        }
    }

    #[test]
    fn test_find_unused_functions_with_regex_pattern_invalid() {
        let db = get_db();
        let result = find_unused_functions(
            &*db,
            Some("[invalid"),
            "default",
            true,
            false,
            false,
            false,
            100,
        );

        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    // ===== Limit tests =====

    #[test]
    fn test_find_unused_functions_limit_2_returns_2() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 2)
            .expect("Query should succeed");

        assert_eq!(unused.len(), 2, "Limit 2 should return exactly 2 results");
    }

    #[test]
    fn test_find_unused_functions_limit_5_returns_5() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 5)
            .expect("Query should succeed");

        assert_eq!(unused.len(), 5, "Limit 5 should return exactly 5 results");
    }

    #[test]
    fn test_find_unused_functions_limit_0_returns_empty() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 0)
            .expect("Query should succeed");

        assert!(unused.is_empty(), "Limit 0 should return empty results");
    }

    #[test]
    fn test_find_unused_functions_limit_100_returns_all_16() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        assert_eq!(unused.len(), 16, "Limit 100 should return all 16 unused functions");
    }

    // ===== Ordering tests =====

    #[test]
    fn test_find_unused_functions_ordered_by_module_name_arity() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        // Results should be ordered by module_name, then name, then arity
        let ordered: Vec<_> = unused
            .iter()
            .map(|f| (f.module.as_str(), f.name.as_str(), f.arity))
            .collect();

        // Expected order (alphabetically by module, then name, then arity):
        let expected = vec![
            ("MyApp.Accounts", "__generated__", 0),
            ("MyApp.Accounts", "__struct__", 0),
            ("MyApp.Accounts", "format_name", 1),
            ("MyApp.Accounts", "validate_email", 1),
            ("MyApp.Cache", "fetch", 1),
            ("MyApp.Controller", "__generated__", 0),
            ("MyApp.Controller", "create", 2),
            ("MyApp.Controller", "format_display", 1),
            ("MyApp.Controller", "index", 2),
            ("MyApp.Controller", "show", 2),
            ("MyApp.Events", "subscribe", 2),
            ("MyApp.Logger", "debug", 1),
            ("MyApp.Metrics", "increment", 1),
            ("MyApp.Repo", "validate", 1),
            ("MyApp.Service", "transform_data", 1),
            ("MyApp.Service", "validate", 1),
        ];

        assert_eq!(ordered, expected, "Results should be ordered by module, name, arity");
    }

    // ===== Combined filter tests =====

    #[test]
    fn test_find_unused_functions_private_and_exclude_generated() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, true, false, true, 100)
            .expect("Query should succeed");

        // Private (3) - none are generated = 3
        assert_eq!(
            unused.len(),
            3,
            "Private + exclude_generated should return 3"
        );

        for func in &unused {
            assert!(func.kind == "defp" || func.kind == "defmacrop");
            assert!(!func.name.starts_with("__"));
        }
    }

    #[test]
    fn test_find_unused_functions_public_and_exclude_generated() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, true, true, 100)
            .expect("Query should succeed");

        // Public (13) - 3 generated (__struct__, __generated__ x2) = 10
        assert_eq!(
            unused.len(),
            10,
            "Public + exclude_generated should return 10 (13 public - 3 generated)"
        );

        // Expected: format_name, fetch, create, format_display, index, show, subscribe, increment, validate x2
        let names: std::collections::HashSet<_> = unused.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("format_name"));
        assert!(names.contains("fetch"));
        assert!(names.contains("index"));
        assert!(names.contains("show"));
        assert!(names.contains("create"));
        assert!(names.contains("format_display"));
        assert!(names.contains("subscribe"));
        assert!(names.contains("increment"));
        assert!(names.contains("validate"));
        assert!(!names.contains("__struct__"));
        assert!(!names.contains("__generated__"));
    }

    #[test]
    fn test_find_unused_functions_controller_private_only() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("MyApp.Controller"),
            "default",
            false,
            true,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        // Controller has only public functions (def), no private
        assert!(
            unused.is_empty(),
            "Controller has no private functions, should return empty"
        );
    }

    #[test]
    fn test_find_unused_functions_accounts_exclude_generated() {
        let db = get_db();
        let unused = find_unused_functions(
            &*db,
            Some("MyApp.Accounts"),
            "default",
            false,
            false,
            false,
            true,
            100,
        )
        .expect("Query should succeed");

        // Accounts has 4 unused, excluding 2 generated (__struct__, __generated__) = 2
        assert_eq!(
            unused.len(),
            2,
            "Accounts with exclude_generated should return 2 (format_name, validate_email)"
        );
        let names: std::collections::HashSet<_> = unused.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("format_name"));
        assert!(names.contains("validate_email"));
    }

    // ===== Edge case tests =====

    #[test]
    fn test_find_unused_functions_module_pattern_case_sensitive() {
        let db = get_db();
        let result_lower = find_unused_functions(
            &*db,
            Some("myapp.controller"),
            "default",
            false,
            false,
            false,
            false,
            100,
        )
        .expect("Query should succeed");

        assert!(
            result_lower.is_empty(),
            "Lowercase pattern should not match CamelCase module"
        );
    }

    #[test]
    fn test_find_unused_functions_result_uniqueness() {
        let db = get_db();
        let unused = find_unused_functions(&*db, None, "default", false, false, false, false, 100)
            .expect("Query should succeed");

        let mut seen = std::collections::HashSet::new();
        for func in &unused {
            let key = format!("{}:{}:{}", func.module, func.name, func.arity);
            assert!(
                !seen.contains(&key),
                "Function {} should not appear multiple times",
                key
            );
            seen.insert(key);
        }
    }
}
