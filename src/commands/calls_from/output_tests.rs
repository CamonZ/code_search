//! Output formatting tests for calls-from command.

#[cfg(test)]
mod tests {
    use super::super::execute::CallsFromResult;
    use crate::queries::calls_from::CallEdge;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Calls from: MyApp.Accounts.get_user

No calls found.";

    const SINGLE_TABLE: &str = "\
Calls from: MyApp.Accounts.get_user

Found 1 call(s):

MyApp.Accounts (lib/my_app/accounts.ex)
  get_user/1 (10:15)
    → MyApp.Repo.get/2 (L12)";

    const MULTIPLE_TABLE: &str = "\
Calls from: MyApp.Accounts

Found 2 call(s):

MyApp.Accounts (lib/my_app/accounts.ex)
  get_user/1 (10:15)
    → MyApp.Repo.get/2 (L12)
  list_users/0 (20:25)
    → MyApp.Repo.all/1 (L22)";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> CallsFromResult {
        CallsFromResult::from_edges(
            "MyApp.Accounts".to_string(),
            "get_user".to_string(),
            vec![],
        )
    }

    #[fixture]
    fn single_result() -> CallsFromResult {
        CallsFromResult::from_edges(
            "MyApp.Accounts".to_string(),
            "get_user".to_string(),
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
    fn multiple_result() -> CallsFromResult {
        CallsFromResult::from_edges(
            "MyApp.Accounts".to_string(),
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
                    caller_module: "MyApp.Accounts".to_string(),
                    caller_function: "list_users".to_string(),
                    caller_arity: 0,
                    caller_kind: String::new(),
                    caller_start_line: 20,
                    caller_end_line: 25,
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "all".to_string(),
                    callee_arity: 1,
                    file: "lib/my_app/accounts.ex".to_string(),
                    line: 22,
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
        fixture_type: CallsFromResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: CallsFromResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: CallsFromResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: CallsFromResult,
        expected: crate::test_utils::load_output_fixture("calls_from", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: CallsFromResult,
        expected: crate::test_utils::load_output_fixture("calls_from", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: CallsFromResult,
        expected: crate::test_utils::load_output_fixture("calls_from", "empty.toon"),
        format: Toon,
    }
}
