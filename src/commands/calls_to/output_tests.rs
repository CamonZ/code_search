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
  MyApp.Accounts.get_user (lib/my_app/accounts.ex:12) -> MyApp.Repo.get/2";

    const MULTIPLE_TABLE: &str = "\
Calls to: MyApp.Repo

Found 2 caller(s):
  MyApp.Accounts.get_user (lib/my_app/accounts.ex:12) -> MyApp.Repo.get/2
  MyApp.Users.update_user (lib/my_app/users.ex:40) -> MyApp.Repo.get/2";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> CallsToResult {
        CallsToResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: "get".to_string(),
            calls: vec![],
        }
    }

    #[fixture]
    fn single_result() -> CallsToResult {
        CallsToResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: "get".to_string(),
            calls: vec![CallEdge {
                project: "default".to_string(),
                caller_module: "MyApp.Accounts".to_string(),
                caller_function: "get_user".to_string(),
                caller_kind: String::new(),
                callee_module: "MyApp.Repo".to_string(),
                callee_function: "get".to_string(),
                callee_arity: 2,
                file: "lib/my_app/accounts.ex".to_string(),
                line: 12,
                call_type: "remote".to_string(),
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> CallsToResult {
        CallsToResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: String::new(),
            calls: vec![
                CallEdge {
                    project: "default".to_string(),
                    caller_module: "MyApp.Accounts".to_string(),
                    caller_function: "get_user".to_string(),
                    caller_kind: String::new(),
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
                    caller_kind: String::new(),
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "get".to_string(),
                    callee_arity: 2,
                    file: "lib/my_app/users.ex".to_string(),
                    line: 40,
                    call_type: "remote".to_string(),
                },
            ],
        }
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
