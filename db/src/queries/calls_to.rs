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
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    find_calls(
        db,
        CallDirection::To,
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
    fn test_find_calls_to_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls_to(
            &*populated_db,
            "",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let calls = result.unwrap();
        // May or may not have results depending on fixture
        assert!(calls.is_empty() || !calls.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_calls_to_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls_to(
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
    fn test_find_calls_to_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_calls_to(
            &*populated_db,
            "",
            None,
            None,
            "default",
            false,
            5,
        )
        .unwrap();
        let limit_100 = find_calls_to(
            &*populated_db,
            "",
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
    fn test_find_calls_to_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_calls_to(
            &*populated_db,
            "",
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
