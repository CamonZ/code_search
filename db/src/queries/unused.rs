use std::error::Error;


use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string, run_query};
use crate::query_builders::{validate_regex_patterns, OptionalConditionBuilder};

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
];

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
