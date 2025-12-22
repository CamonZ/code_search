//! Output formatting tests for calls-to command.

#[cfg(test)]
mod tests {
    use super::super::execute::CalleeFunction;
    use db::types::{Call, FunctionRef, ModuleGroupResult};
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
    ← @ L12 MyApp.Accounts.get_user/1 (accounts.ex:L10:15)";

    const MULTIPLE_TABLE: &str = "\
Calls to: MyApp.Repo

Found 2 caller(s):

MyApp.Repo
  get/2
    ← @ L12 MyApp.Accounts.get_user/1 (accounts.ex:L10:15)
    ← @ L40 MyApp.Users.update_user/1 (users.ex:L35:45)";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleGroupResult<CalleeFunction> {
        ModuleGroupResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: Some("get".to_string()),
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleGroupResult<CalleeFunction> {
        use db::types::ModuleGroup;

        let callee_func = CalleeFunction {
            name: "get".to_string(),
            arity: 2,
            callers: vec![Call {
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
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: Some("get".to_string()),
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Repo".to_string(),
                file: String::new(),
                entries: vec![callee_func],
                function_count: None,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleGroupResult<CalleeFunction> {
        use db::types::ModuleGroup;

        let callee_func = CalleeFunction {
            name: "get".to_string(),
            arity: 2,
            callers: vec![
                Call {
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
                },
                Call {
                    caller: FunctionRef::with_definition(
                        "MyApp.Users",
                        "update_user",
                        1,
                        "",
                        "lib/my_app/users.ex",
                        35,
                        45,
                    ),
                    callee: FunctionRef::new("MyApp.Repo", "get", 2),
                    line: 40,
                    call_type: Some("remote".to_string()),
                    depth: None,
                },
            ],
        };

        ModuleGroupResult {
            module_pattern: "MyApp.Repo".to_string(),
            function_pattern: None,
            total_items: 2,
            items: vec![ModuleGroup {
                name: "MyApp.Repo".to_string(),
                file: String::new(),
                entries: vec![callee_func],
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
        fixture_type: ModuleGroupResult<CalleeFunction>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleGroupResult<CalleeFunction>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleGroupResult<CalleeFunction>,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleGroupResult<CalleeFunction>,
        expected: db::test_utils::load_output_fixture("calls_to", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleGroupResult<CalleeFunction>,
        expected: db::test_utils::load_output_fixture("calls_to", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<CalleeFunction>,
        expected: db::test_utils::load_output_fixture("calls_to", "empty.toon"),
        format: Toon,
    }
}
