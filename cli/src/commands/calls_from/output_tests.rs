//! Output formatting tests for calls-from command.

#[cfg(test)]
mod tests {
    use super::super::execute::CallerFunction;
    use db::types::{Call, FunctionRef, ModuleGroupResult};
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
    → @ L12 MyApp.Repo.get/2";

    const MULTIPLE_TABLE: &str = "\
Calls from: MyApp.Accounts

Found 2 call(s):

MyApp.Accounts (lib/my_app/accounts.ex)
  get_user/1 (10:15)
    → @ L12 MyApp.Repo.get/2
  list_users/0 (20:25)
    → @ L22 MyApp.Repo.all/1";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleGroupResult<CallerFunction> {
        ModuleGroupResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: Some("get_user".to_string()),
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleGroupResult<CallerFunction> {
        use db::types::ModuleGroup;

        let caller_func = CallerFunction {
            name: "get_user".to_string(),
            arity: 1,
            kind: String::new(),
            start_line: 10,
            end_line: 15,
            calls: vec![Call {
                caller: FunctionRef::with_definition(
                    "MyApp.Accounts",
                    "get_user",
                    1,
                    "",
                    "lib/my_app/accounts.ex",
                    10,
                    15,
                ),
                callee: FunctionRef::new("MyApp.Repo", "get", 2),
                line: 12,
                call_type: Some("remote".to_string()),
                depth: None,
            }],
        };

        ModuleGroupResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: Some("get_user".to_string()),
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                entries: vec![caller_func],
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleGroupResult<CallerFunction> {
        use db::types::ModuleGroup;

        let caller_func1 = CallerFunction {
            name: "get_user".to_string(),
            arity: 1,
            kind: String::new(),
            start_line: 10,
            end_line: 15,
            calls: vec![Call {
                caller: FunctionRef::with_definition(
                    "MyApp.Accounts",
                    "get_user",
                    1,
                    "",
                    "lib/my_app/accounts.ex",
                    10,
                    15,
                ),
                callee: FunctionRef::new("MyApp.Repo", "get", 2),
                line: 12,
                call_type: Some("remote".to_string()),
                depth: None,
            }],
        };

        let caller_func2 = CallerFunction {
            name: "list_users".to_string(),
            arity: 0,
            kind: String::new(),
            start_line: 20,
            end_line: 25,
            calls: vec![Call {
                caller: FunctionRef::with_definition(
                    "MyApp.Accounts",
                    "list_users",
                    0,
                    "",
                    "lib/my_app/accounts.ex",
                    20,
                    25,
                ),
                callee: FunctionRef::new("MyApp.Repo", "all", 1),
                line: 22,
                call_type: Some("remote".to_string()),
                depth: None,
            }],
        };

        ModuleGroupResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: None,
            total_items: 2,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                entries: vec![caller_func1, caller_func2],
                function_count: None,
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<CallerFunction>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleGroupResult<CallerFunction>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleGroupResult<CallerFunction>,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleGroupResult<CallerFunction>,
        expected: db::test_utils::load_output_fixture("calls_from", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleGroupResult<CallerFunction>,
        expected: db::test_utils::load_output_fixture("calls_from", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<CallerFunction>,
        expected: db::test_utils::load_output_fixture("calls_from", "empty.toon"),
        format: Toon,
    }
}
