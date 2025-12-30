//! Find incoming calls to functions.
//!
//! This is a convenience wrapper around [`super::calls::find_calls`] with
//! [`CallDirection::To`](super::calls::CallDirection::To).

use std::error::Error;

use super::calls::{find_calls, CallDirection};
use crate::backend::Database;
use crate::types::Call;

pub fn find_calls_to(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    find_calls(
        db,
        CallDirection::To,
        module_pattern,
        function_pattern,
        arity,
        use_regex,
        limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_calls_to_returns_ok() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_to(
            &*db,
            "module_a",
            None,
            None,
            false,
            100,
        );

        assert!(result.is_ok(), "Should execute successfully");
    }

    #[test]
    fn test_find_calls_to_empty_for_nonexistent() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_to(
            &*db,
            "NonExistent",
            None,
            None,
            false,
            100,
        );

        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty(), "Non-existent module should return empty");
    }

    #[test]
    fn test_find_calls_to_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_2 = find_calls_to(
            &*db,
            "MyApp.Accounts",
            None,
            None,
            false,
            2,
        )
        .unwrap_or_default();

        assert!(limit_2.len() <= 2, "Limit of 2 should be respected");
    }

    #[test]
    fn test_find_calls_to_with_function_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_to(
            &*db,
            "module_a",
            Some("bar"),
            None,
            false,
            100,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_find_calls_to_with_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_calls_to(
            &*db,
            "[invalid",
            None,
            None,
            true,
            100,
        );

        assert!(result.is_err(), "Should reject invalid regex");
    }
}
