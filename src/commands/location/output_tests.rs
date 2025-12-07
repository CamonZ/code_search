//! Output formatting tests for location command.

#[cfg(test)]
mod tests {
    use super::super::execute::{FunctionLocation, LocationResult};
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
  [default] MyApp.Accounts.get_user/1 (def)
       lib/my_app/accounts.ex:10:15";

    const MULTIPLE_TABLE: &str = "\
Location: MyApp.*.user

Found 2 location(s):
  [default] MyApp.Accounts.get_user/1 (def)
       lib/my_app/accounts.ex:10:15
  [default] MyApp.Users.create_user/1 (def)
       lib/my_app/users.ex:5:12";

    const SINGLE_JSON: &str = r#"{
  "module_pattern": "MyApp.Accounts",
  "function_pattern": "get_user",
  "locations": [
    {
      "project": "default",
      "file": "lib/my_app/accounts.ex",
      "start_line": 10,
      "end_line": 15,
      "module": "MyApp.Accounts",
      "kind": "def",
      "name": "get_user",
      "arity": 1
    }
  ]
}"#;

    const SINGLE_TOON: &str = "\
function_pattern: get_user
locations[1]{arity,end_line,file,kind,module,name,project,start_line}:
  1,15,lib/my_app/accounts.ex,def,MyApp.Accounts,get_user,default,10
module_pattern: MyApp.Accounts";

    const EMPTY_TOON: &str = "\
function_pattern: foo
locations[0]:
module_pattern: MyApp";

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
        expected: SINGLE_JSON,
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_result,
        fixture_type: LocationResult,
        expected: SINGLE_TOON,
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: LocationResult,
        expected: EMPTY_TOON,
        format: Toon,
    }
}
