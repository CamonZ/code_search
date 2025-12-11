//! Output formatting tests for function command.

#[cfg(test)]
mod tests {
    use super::super::execute::FuncSig;
    use crate::types::{ModuleGroupResult, ModuleGroup};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Function: MyApp.Accounts.get_user

No functions found.";

    const SINGLE_TABLE: &str = "\
Function: MyApp.Accounts.get_user

Found 1 signature(s) in 1 module(s):

MyApp.Accounts:
  get_user/1
    args: integer()
    returns: User.t() | nil";

    const MULTIPLE_TABLE: &str = "\
Function: MyApp.Accounts.get_user

Found 2 signature(s) in 1 module(s):

MyApp.Accounts:
  get_user/1
    args: integer()
    returns: User.t() | nil
  get_user/2
    args: integer(), keyword()
    returns: User.t() | nil";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> ModuleGroupResult<FuncSig> {
        ModuleGroupResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: Some("get_user".to_string()),
            total_items: 0,
            items: vec![],
        }
    }

    #[fixture]
    fn single_result() -> ModuleGroupResult<FuncSig> {
        ModuleGroupResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: Some("get_user".to_string()),
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![FuncSig {
                    name: "get_user".to_string(),
                    arity: 1,
                    args: "integer()".to_string(),
                    return_type: "User.t() | nil".to_string(),
                }],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> ModuleGroupResult<FuncSig> {
        ModuleGroupResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: Some("get_user".to_string()),
            total_items: 2,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: String::new(),
                entries: vec![
                    FuncSig {
                        name: "get_user".to_string(),
                        arity: 1,
                        args: "integer()".to_string(),
                        return_type: "User.t() | nil".to_string(),
                    },
                    FuncSig {
                        name: "get_user".to_string(),
                        arity: 2,
                        args: "integer(), keyword()".to_string(),
                        return_type: "User.t() | nil".to_string(),
                    },
                ],
            }],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<FuncSig>,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: ModuleGroupResult<FuncSig>,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: ModuleGroupResult<FuncSig>,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: ModuleGroupResult<FuncSig>,
        expected: crate::test_utils::load_output_fixture("function", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: ModuleGroupResult<FuncSig>,
        expected: crate::test_utils::load_output_fixture("function", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: ModuleGroupResult<FuncSig>,
        expected: crate::test_utils::load_output_fixture("function", "empty.toon"),
        format: Toon,
    }
}
