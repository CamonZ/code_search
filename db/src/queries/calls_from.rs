//! Find outgoing calls from functions.
//!
//! This is a convenience wrapper around [`super::calls::find_calls`] with
//! [`CallDirection::From`](super::calls::CallDirection::From).

use std::error::Error;

use super::calls::{find_calls, CallDirection};
use crate::backend::Database;
use crate::types::Call;

pub fn find_calls_from(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    find_calls(
        db,
        CallDirection::From,
        module_pattern,
        function_pattern,
        arity,
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
    fn test_find_calls_from_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls_from(
            &*populated_db,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(!calls.is_empty(), "Should find outgoing calls");
    }

    #[rstest]
    fn test_find_calls_from_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls_from(
            &*populated_db,
            "NonExistent",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty(), "Non-existent module should return no calls");
    }

    #[rstest]
    fn test_find_calls_from_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_calls_from(
            &*populated_db,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            5,
        )
        .unwrap();
        let limit_100 = find_calls_from(
            &*populated_db,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            100,
        )
        .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_calls_from_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls_from(
            &*populated_db,
            "MyApp.Controller",
            None,
            None,
            "nonexistent",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty(), "Non-existent project should return no results");
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_calls_from_returns_ok() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_from(
            &*db,
            "module_a",
            None,
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Should execute successfully");
    }

    #[test]
    fn test_find_calls_from_empty_for_nonexistent() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_from(
            &*db,
            "NonExistent",
            None,
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty(), "Non-existent module should return empty");
    }

    #[test]
    fn test_find_calls_from_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_2 = find_calls_from(
            &*db,
            "MyApp.Controller",
            None,
            None,
            "default",
            false,
            2,
        )
        .unwrap_or_default();

        assert!(limit_2.len() <= 2, "Limit of 2 should be respected");
    }

    #[test]
    fn test_find_calls_from_with_function_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_from(
            &*db,
            "module_a",
            Some("foo"),
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_find_calls_from_with_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_from(
            &*db,
            "[invalid",
            None,
            None,
            "default",
            true,
            100,
        );

        assert!(result.is_err(), "Should reject invalid regex");
    }
}
