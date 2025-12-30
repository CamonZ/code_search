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
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    query_dependencies(
        db,
        DependencyDirection::Incoming,
        module_pattern,
        use_regex,
        limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_dependents_returns_results() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependents(&*db, "MyApp.Notifier", false, 100);

        assert!(result.is_ok(), "Query should succeed");
        let calls = result.unwrap();
        // MyApp.Notifier is called by MyApp.Service, MyApp.Controller, and MyApp.Cache (Cycle C)
        assert_eq!(calls.len(), 3, "Should find 3 incoming dependencies");

        // Verify callers (order may vary)
        let callers: Vec<&str> = calls.iter().map(|c| c.caller.module.as_ref()).collect();
        assert!(
            callers.contains(&"MyApp.Service") && callers.contains(&"MyApp.Controller") && callers.contains(&"MyApp.Cache"),
            "Should find calls from Service, Controller, and Cache"
        );
        for call in &calls {
            assert_eq!(call.callee.module.as_ref(), "MyApp.Notifier");
        }
    }

    #[test]
    fn test_find_dependents_empty_for_nonexistent() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependents(&*db, "NonExistent", false, 100);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_find_dependents_excludes_self_references() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependents(&*db, "MyApp.Notifier", false, 100).unwrap();

        for call in &result {
            assert_ne!(
                call.caller.module, call.callee.module,
                "Self-references should be excluded"
            );
        }
    }

    #[test]
    fn test_find_dependents_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependents(&*db, "[invalid", true, 100);

        assert!(result.is_err(), "Should reject invalid regex");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Invalid regex"),
            "Error should mention invalid regex: {}",
            err
        );
    }

    #[test]
    fn test_find_dependents_non_regex_mode() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Invalid regex pattern should succeed in non-regex mode (treated as literal)
        let result = find_dependents(&*db, "[invalid", false, 100);

        assert!(result.is_ok(), "Should succeed in non-regex mode");
    }

    #[test]
    fn test_find_dependents_with_regex_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependents(&*db, "^MyApp\\.Accounts$", true, 100);

        assert!(result.is_ok());
        let calls = result.unwrap();
        // All calls should target MyApp.Accounts
        for call in &calls {
            assert_eq!(call.callee.module.as_ref(), "MyApp.Accounts");
        }
    }

    #[test]
    fn test_find_dependents_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_1 = find_dependents(&*db, "MyApp.Accounts", false, 1)
            .unwrap_or_default();
        let limit_100 = find_dependents(&*db, "MyApp.Accounts", false, 100)
            .unwrap_or_default();

        assert!(limit_1.len() <= 1, "Limit of 1 should be respected");
        assert!(limit_1.len() <= limit_100.len());
    }
}
