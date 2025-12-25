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
