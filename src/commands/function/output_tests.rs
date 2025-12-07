//! Output formatting tests for function command.

#[cfg(test)]
mod tests {
    use super::super::execute::{FunctionResult, FunctionSignature};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Function: MyApp.Accounts.get_user

No functions found.";

    const SINGLE_TABLE: &str = "\
Function: MyApp.Accounts.get_user

Found 1 signature(s):
  [default] MyApp.Accounts.get_user/1
       args: integer()
       returns: User.t() | nil";

    const MULTIPLE_TABLE: &str = "\
Function: MyApp.Accounts.get_user

Found 2 signature(s):
  [default] MyApp.Accounts.get_user/1
       args: integer()
       returns: User.t() | nil
  [default] MyApp.Accounts.get_user/2
       args: integer(), keyword()
       returns: User.t() | nil";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> FunctionResult {
        FunctionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            functions: vec![],
        }
    }

    #[fixture]
    fn single_result() -> FunctionResult {
        FunctionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            functions: vec![FunctionSignature {
                project: "default".to_string(),
                module: "MyApp.Accounts".to_string(),
                name: "get_user".to_string(),
                arity: 1,
                args: "integer()".to_string(),
                return_type: "User.t() | nil".to_string(),
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> FunctionResult {
        FunctionResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            functions: vec![
                FunctionSignature {
                    project: "default".to_string(),
                    module: "MyApp.Accounts".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                    args: "integer()".to_string(),
                    return_type: "User.t() | nil".to_string(),
                },
                FunctionSignature {
                    project: "default".to_string(),
                    module: "MyApp.Accounts".to_string(),
                    name: "get_user".to_string(),
                    arity: 2,
                    args: "integer(), keyword()".to_string(),
                    return_type: "User.t() | nil".to_string(),
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
        fixture_type: FunctionResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: FunctionResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: FunctionResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: FunctionResult,
        expected: crate::test_utils::load_output_fixture("function", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: FunctionResult,
        expected: crate::test_utils::load_output_fixture("function", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: FunctionResult,
        expected: crate::test_utils::load_output_fixture("function", "empty.toon"),
        format: Toon,
    }
}
