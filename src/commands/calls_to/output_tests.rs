//! Output formatting tests for calls-to command.

#[cfg(test)]
mod tests {
    use super::super::execute::CallsToResult;
    use crate::queries::calls_to::CallEdge;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Calls to: MyApp.Repo.get

No callers found.";

    const SINGLE_TABLE: &str = "\
Calls to: MyApp.Repo.get

Found 1 caller(s):

MyApp.Repo
  get/2
    ← MyApp.Accounts.get_user/1 (lib/my_app/accounts.ex:10:15) (L12)";

    const MULTIPLE_TABLE: &str = "\
Calls to: MyApp.Repo

Found 2 caller(s):

MyApp.Repo
  get/2
    ← MyApp.Accounts.get_user/1 (lib/my_app/accounts.ex:10:15) (L12)
    ← MyApp.Users.update_user/1 (lib/my_app/users.ex:35:45) (L40)";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> CallsToResult {
        CallsToResult::from_edges(
            "MyApp.Repo".to_string(),
            "get".to_string(),
            vec![],
        )
    }

    #[fixture]
    fn single_result() -> CallsToResult {
        CallsToResult::from_edges(
            "MyApp.Repo".to_string(),
            "get".to_string(),
            vec![CallEdge {
                project: "default".to_string(),
                caller_module: "MyApp.Accounts".to_string(),
                caller_function: "get_user".to_string(),
                caller_arity: 1,
                caller_kind: String::new(),
                caller_start_line: 10,
                caller_end_line: 15,
                callee_module: "MyApp.Repo".to_string(),
                callee_function: "get".to_string(),
                callee_arity: 2,
                file: "lib/my_app/accounts.ex".to_string(),
                line: 12,
                call_type: "remote".to_string(),
            }],
        )
    }

    #[fixture]
    fn multiple_result() -> CallsToResult {
        CallsToResult::from_edges(
            "MyApp.Repo".to_string(),
            String::new(),
            vec![
                CallEdge {
                    project: "default".to_string(),
                    caller_module: "MyApp.Accounts".to_string(),
                    caller_function: "get_user".to_string(),
                    caller_arity: 1,
                    caller_kind: String::new(),
                    caller_start_line: 10,
                    caller_end_line: 15,
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "get".to_string(),
                    callee_arity: 2,
                    file: "lib/my_app/accounts.ex".to_string(),
                    line: 12,
                    call_type: "remote".to_string(),
                },
                CallEdge {
                    project: "default".to_string(),
                    caller_module: "MyApp.Users".to_string(),
                    caller_function: "update_user".to_string(),
                    caller_arity: 1,
                    caller_kind: String::new(),
                    caller_start_line: 35,
                    caller_end_line: 45,
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "get".to_string(),
                    callee_arity: 2,
                    file: "lib/my_app/users.ex".to_string(),
                    line: 40,
                    call_type: "remote".to_string(),
                },
            ],
        )
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: CallsToResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: CallsToResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: CallsToResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: CallsToResult,
        expected: crate::test_utils::load_output_fixture("calls_to", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: CallsToResult,
        expected: crate::test_utils::load_output_fixture("calls_to", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: CallsToResult,
        expected: crate::test_utils::load_output_fixture("calls_to", "empty.toon"),
        format: Toon,
    }
}
