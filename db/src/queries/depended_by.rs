//! Find incoming module dependencies.
//!
//! This is a convenience wrapper around [`super::dependencies::find_dependencies`] with
//! [`DependencyDirection::Incoming`](super::dependencies::DependencyDirection::Incoming).

use std::error::Error;

use super::dependencies::{find_dependencies as query_dependencies, DependencyDirection};
use crate::backend::Database;
use crate::types::Call;
use crate::query_builders::validate_regex_patterns;

pub fn find_dependents(
    db: &dyn Database,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    query_dependencies(
        db,
        DependencyDirection::Incoming,
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
    fn test_find_dependents_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependents(&*populated_db, "MyApp.Accounts", "default", false, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        // MyApp.Accounts should be depended on by other modules
        assert!(!calls.is_empty(), "MyApp.Accounts should have incoming dependencies");
    }

    #[rstest]
    fn test_find_dependents_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependents(&*populated_db, "NonExistent", "default", false, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        // Non-existent module should have no dependents
        assert!(calls.is_empty());
    }

    #[rstest]
    fn test_find_dependents_excludes_self_references(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependents(&*populated_db, "MyApp.Accounts", "default", false, 100)
            .unwrap();

        // All calls should be from other modules, not self
        for call in &result {
            assert_ne!(
                call.caller.module, call.callee.module,
                "Self-references should be excluded"
            );
        }
    }

    #[rstest]
    fn test_find_dependents_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_dependents(&*populated_db, "MyApp.Accounts", "default", false, 5)
            .unwrap();
        let limit_100 = find_dependents(&*populated_db, "MyApp.Accounts", "default", false, 100)
            .unwrap();

        // Smaller limit should return fewer results
        assert!(limit_5.len() <= limit_100.len());
        assert!(limit_5.len() <= 5);
    }

    #[rstest]
    fn test_find_dependents_with_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependents(&*populated_db, "^MyApp\\.Accounts$", "default", true, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        // All calls should target MyApp.Accounts module
        for call in &calls {
            assert_eq!(call.callee.module.as_ref(), "MyApp.Accounts");
        }
    }

    #[rstest]
    fn test_find_dependents_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependents(&*populated_db, "[invalid", "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_dependents_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependents(&*populated_db, "Accounts", "nonexistent", false, 100);
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty());
    }

    #[rstest]
    fn test_find_dependents_non_regex_mode(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependents(&*populated_db, "[invalid", "default", false, 100);
        // Should succeed in non-regex mode (treated as literal string)
        assert!(result.is_ok());
    }
}
