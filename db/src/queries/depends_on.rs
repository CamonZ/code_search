//! Find outgoing module dependencies.
//!
//! This is a convenience wrapper around [`super::dependencies::find_dependencies`] with
//! [`DependencyDirection::Outgoing`](super::dependencies::DependencyDirection::Outgoing).

use std::error::Error;

use super::dependencies::{find_dependencies as query_dependencies, DependencyDirection};
use crate::backend::Database;
use crate::types::Call;
use crate::query_builders::validate_regex_patterns;

pub fn find_dependencies(
    db: &dyn Database,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    query_dependencies(
        db,
        DependencyDirection::Outgoing,
        module_pattern,
        project,
        use_regex,
        limit,
    )
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
    fn test_find_dependencies_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(&*populated_db, "MyApp.Controller", "default", false, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        // MyApp.Controller should depend on other modules
        assert!(!calls.is_empty(), "MyApp.Controller should have outgoing dependencies");
    }

    #[rstest]
    fn test_find_dependencies_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(&*populated_db, "NonExistent", "default", false, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        // Non-existent module should have no dependencies
        assert!(calls.is_empty());
    }

    #[rstest]
    fn test_find_dependencies_excludes_self_references(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(&*populated_db, "MyApp.Controller", "default", false, 100)
            .unwrap();

        // All calls should be to other modules, not self
        for call in &result {
            assert_ne!(
                call.caller.module, call.callee.module,
                "Self-references should be excluded"
            );
        }
    }

    #[rstest]
    fn test_find_dependencies_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_dependencies(&*populated_db, "MyApp.Controller", "default", false, 5)
            .unwrap();
        let limit_100 = find_dependencies(&*populated_db, "MyApp.Controller", "default", false, 100)
            .unwrap();

        // Smaller limit should return fewer results
        assert!(limit_5.len() <= limit_100.len());
        assert!(limit_5.len() <= 5);
    }

    #[rstest]
    fn test_find_dependencies_with_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(&*populated_db, "^MyApp\\.Controller$", "default", true, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        // All calls should originate from MyApp.Controller module
        for call in &calls {
            assert_eq!(call.caller.module.as_ref(), "MyApp.Controller");
        }
    }

    #[rstest]
    fn test_find_dependencies_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(&*populated_db, "[invalid", "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_dependencies_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(&*populated_db, "Controller", "nonexistent", false, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty());
    }

    #[rstest]
    fn test_find_dependencies_non_regex_mode(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(&*populated_db, "[invalid", "default", false, 100);
        // Should succeed in non-regex mode (treated as literal string)
        assert!(result.is_ok());
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_dependencies_returns_results() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_dependencies(&*db, "module_a", "default", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let calls = result.unwrap();
        // module_a.foo calls module_b.baz (cross-module dependency)
        assert_eq!(calls.len(), 1, "Should find 1 outgoing dependency");
        assert_eq!(calls[0].caller.module.as_ref(), "module_a");
        assert_eq!(calls[0].callee.module.as_ref(), "module_b");
    }

    #[test]
    fn test_find_dependencies_empty_for_nonexistent() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_dependencies(&*db, "NonExistent", "default", false, 100);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_find_dependencies_excludes_self_references() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_dependencies(&*db, "module_a", "default", false, 100).unwrap();

        for call in &result {
            assert_ne!(
                call.caller.module, call.callee.module,
                "Self-references should be excluded"
            );
        }
    }

    #[test]
    fn test_find_dependencies_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db();

        let result = find_dependencies(&*db, "[invalid", "default", true, 100);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Invalid regex"),
            "Error should mention invalid regex: {}",
            err
        );
    }

    #[test]
    fn test_find_dependencies_non_regex_mode() {
        let db = crate::test_utils::surreal_call_graph_db();

        // Invalid regex pattern should succeed in non-regex mode (treated as literal)
        let result = find_dependencies(&*db, "[invalid", "default", false, 100);

        assert!(result.is_ok(), "Should succeed in non-regex mode");
    }

    #[test]
    fn test_find_dependencies_with_regex_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependencies(&*db, "^MyApp\\.Controller$", "default", true, 100);

        assert!(result.is_ok());
        let calls = result.unwrap();
        // All calls should originate from MyApp.Controller
        for call in &calls {
            assert_eq!(call.caller.module.as_ref(), "MyApp.Controller");
        }
    }

    #[test]
    fn test_find_dependencies_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_1 = find_dependencies(&*db, "MyApp.Controller", "default", false, 1)
            .unwrap_or_default();
        let limit_100 = find_dependencies(&*db, "MyApp.Controller", "default", false, 100)
            .unwrap_or_default();

        assert!(limit_1.len() <= 1, "Limit of 1 should be respected");
        assert!(limit_1.len() <= limit_100.len());
    }
}
