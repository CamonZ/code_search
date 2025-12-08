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
  [default] MyApp.Accounts.get_user (lib/my_app/accounts.ex:12) -> MyApp.Repo.get/2";

    const MULTIPLE_TABLE: &str = "\
Calls from: MyApp.Accounts

Found 2 call(s):
  [default] MyApp.Accounts.get_user (lib/my_app/accounts.ex:12) -> MyApp.Repo.get/2
  [default] MyApp.Accounts.list_users (lib/my_app/accounts.ex:22) -> MyApp.Repo.all/1";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> CallsFromResult {
        CallsFromResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            calls: vec![],
        }
    }

    #[fixture]
    fn single_result() -> CallsFromResult {
        CallsFromResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            calls: vec![CallEdge {
                project: "default".to_string(),
                caller_module: "MyApp.Accounts".to_string(),
                caller_function: "get_user".to_string(),
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
    fn multiple_result() -> CallsFromResult {
        CallsFromResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: String::new(),
            calls: vec![
                CallEdge {
                    project: "default".to_string(),
                    caller_module: "MyApp.Accounts".to_string(),
                    caller_function: "get_user".to_string(),
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
                    callee_module: "MyApp.Repo".to_string(),
                    callee_function: "all".to_string(),
                    callee_arity: 1,
                    file: "lib/my_app/accounts.ex".to_string(),
                    line: 22,
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
