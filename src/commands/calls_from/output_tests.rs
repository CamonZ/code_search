//! Output formatting tests for calls-from command.

#[cfg(test)]
mod tests {
    use super::super::execute::{CallEdge, CallsFromResult};
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

    const SINGLE_JSON: &str = r#"{
  "module_pattern": "MyApp.Accounts",
  "function_pattern": "get_user",
  "calls": [
    {
      "project": "default",
      "caller_module": "MyApp.Accounts",
      "caller_function": "get_user",
      "callee_module": "MyApp.Repo",
      "callee_function": "get",
      "callee_arity": 2,
      "file": "lib/my_app/accounts.ex",
      "line": 12,
      "call_type": "remote"
    }
  ]
}"#;

    const SINGLE_TOON: &str = "\
calls[1]{call_type,callee_arity,callee_function,callee_module,caller_function,caller_module,file,line,project}:
  remote,2,get,MyApp.Repo,get_user,MyApp.Accounts,lib/my_app/accounts.ex,12,default
function_pattern: get_user
module_pattern: MyApp.Accounts";

    const EMPTY_TOON: &str = "\
calls[0]:
function_pattern: get_user
module_pattern: MyApp.Accounts";

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
        expected: SINGLE_JSON,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: CallsFromResult,
        expected: SINGLE_TOON,
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: CallsFromResult,
        expected: EMPTY_TOON,
        format: Toon,
    }
}
