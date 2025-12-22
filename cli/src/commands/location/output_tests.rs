//! Output formatting tests for location command.

#[cfg(test)]
mod tests {
    use super::super::execute::{LocationClause, LocationFunction, LocationModule, LocationResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Location: MyApp.foo

No locations found.";

    const SINGLE_TABLE: &str = "\
Location: MyApp.Accounts.get_user

Found 1 clause(s) in 1 function(s) across 1 module(s):

MyApp.Accounts:
  get_user/1 [def] (lib/my_app/accounts.ex)
    L10:15";

    const MULTIPLE_TABLE: &str = "\
Location: MyApp.*.user

Found 2 clause(s) in 2 function(s) across 2 module(s):

MyApp.Accounts:
  get_user/1 [def] (lib/my_app/accounts.ex)
    L10:15
MyApp.Users:
  create_user/1 [def] (lib/my_app/users.ex)
    L5:12";


    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp".to_string(),
            function_pattern: "foo".to_string(),
            total_clauses: 0,
            modules: vec![],
        }
    }

    #[fixture]
    fn single_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            total_clauses: 1,
            modules: vec![LocationModule {
                name: "MyApp.Accounts".to_string(),
                functions: vec![LocationFunction {
                    name: "get_user".to_string(),
                    arity: 1,
                    kind: "def".to_string(),
                    file: "lib/my_app/accounts.ex".to_string(),
                    clauses: vec![LocationClause {
                        line: 10,
                        start_line: 10,
                        end_line: 15,
                        pattern: String::new(),
                        guard: String::new(),
                    }],
                }],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp.*".to_string(),
            function_pattern: "user".to_string(),
            total_clauses: 2,
            modules: vec![
                LocationModule {
                    name: "MyApp.Accounts".to_string(),
                    functions: vec![LocationFunction {
                        name: "get_user".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        file: "lib/my_app/accounts.ex".to_string(),
                        clauses: vec![LocationClause {
                            line: 10,
                            start_line: 10,
                            end_line: 15,
                            pattern: String::new(),
                            guard: String::new(),
                        }],
                    }],
                },
                LocationModule {
                    name: "MyApp.Users".to_string(),
                    functions: vec![LocationFunction {
                        name: "create_user".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        file: "lib/my_app/users.ex".to_string(),
                        clauses: vec![LocationClause {
                            line: 5,
                            start_line: 5,
                            end_line: 12,
                            pattern: String::new(),
                            guard: String::new(),
                        }],
                    }],
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
