//! Output formatting tests for location command.

#[cfg(test)]
mod tests {
    use super::super::execute::LocationResult;
    use crate::queries::location::FunctionLocation;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Location: MyApp.foo

No locations found.";

    const SINGLE_TABLE: &str = "\
Location: MyApp.Accounts.get_user

Found 1 location(s):
  MyApp.Accounts.get_user/1 (def)
       lib/my_app/accounts.ex:10:15";

    const MULTIPLE_TABLE: &str = "\
Location: MyApp.*.user

Found 2 location(s):
  MyApp.Accounts.get_user/1 (def)
       lib/my_app/accounts.ex:10:15
  MyApp.Users.create_user/1 (def)
       lib/my_app/users.ex:5:12";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp".to_string(),
            function_pattern: "foo".to_string(),
            locations: vec![],
        }
    }

    #[fixture]
    fn single_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            locations: vec![FunctionLocation {
                project: "default".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                start_line: 10,
                end_line: 15,
                module: "MyApp.Accounts".to_string(),
                kind: "def".to_string(),
                name: "get_user".to_string(),
                arity: 1,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp.*".to_string(),
            function_pattern: "user".to_string(),
            locations: vec![
                FunctionLocation {
                    project: "default".to_string(),
                    file: "lib/my_app/accounts.ex".to_string(),
                    start_line: 10,
                    end_line: 15,
                    module: "MyApp.Accounts".to_string(),
                    kind: "def".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                },
                FunctionLocation {
                    project: "default".to_string(),
                    file: "lib/my_app/users.ex".to_string(),
                    start_line: 5,
                    end_line: 12,
                    module: "MyApp.Users".to_string(),
                    kind: "def".to_string(),
                    name: "create_user".to_string(),
                    arity: 1,
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
        fixture_type: LocationResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single,
        fixture: single_result,
        fixture_type: LocationResult,
        expected: SINGLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple,
        fixture: multiple_result,
        fixture_type: LocationResult,
        expected: MULTIPLE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_result,
        fixture_type: LocationResult,
        expected: crate::test_utils::load_output_fixture("location", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: LocationResult,
        expected: crate::test_utils::load_output_fixture("location", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: LocationResult,
        expected: crate::test_utils::load_output_fixture("location", "empty.toon"),
        format: Toon,
    }
}
