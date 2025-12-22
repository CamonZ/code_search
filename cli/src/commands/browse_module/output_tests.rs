//! Output formatting tests for browse-module command.

#[cfg(test)]
mod tests {
    use super::super::execute::{BrowseModuleResult, Definition};
    use super::super::DefinitionKind;
    use db::queries::structs::FieldInfo;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Definitions in NonExistent (project: default)

No definitions found.
";

    const FUNCTIONS_ONLY_TABLE: &str = "\
Definitions in MyApp.Accounts (kind: functions, project: default)

Found 2 definition(s):

  MyApp.Accounts:
    L10-15  [def] get_user/1
    L24-28  [def] list_users/0

";

    const MIXED_TYPES_TABLE: &str = "\
Definitions in MyApp.Accounts (project: default)

Found 3 definition(s):

  MyApp.Accounts:
    L5    [type] user
           @type user() :: %{id: integer()}
    L8    [spec] get_user/1
           @spec get_user(integer()) :: User.t()
    L10-15  [def] get_user/1

";

    const STRUCT_TABLE: &str = "\
Definitions in MyApp.User (kind: structs, project: default)

Found 1 definition(s):

  MyApp.User:
    [struct] MyApp.User with 2 fields
           - id: integer() (required)
           - name: String.t() (optional)

";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> BrowseModuleResult {
        BrowseModuleResult {
            search_term: "NonExistent".to_string(),
            kind_filter: None,
            project: "default".to_string(),
            total_items: 0,
            definitions: vec![],
        }
    }

    #[fixture]
    fn functions_only_result() -> BrowseModuleResult {
        BrowseModuleResult {
            search_term: "MyApp.Accounts".to_string(),
            kind_filter: Some(DefinitionKind::Functions),
            project: "default".to_string(),
            total_items: 2,
            definitions: vec![
                Definition::Function {
                    module: "MyApp.Accounts".to_string(),
                    file: "lib/accounts.ex".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                    line: 10,
                    start_line: 10,
                    end_line: 15,
                    kind: "def".to_string(),
                    args: String::new(),
                    return_type: String::new(),
                    pattern: String::new(),
                    guard: String::new(),
                },
                Definition::Function {
                    module: "MyApp.Accounts".to_string(),
                    file: "lib/accounts.ex".to_string(),
                    name: "list_users".to_string(),
                    arity: 0,
                    line: 24,
                    start_line: 24,
                    end_line: 28,
                    kind: "def".to_string(),
                    args: String::new(),
                    return_type: String::new(),
                    pattern: String::new(),
                    guard: String::new(),
                },
            ],
        }
    }

    #[fixture]
    fn mixed_types_result() -> BrowseModuleResult {
        // Definitions are sorted by module then line - so L5, L8, L10
        BrowseModuleResult {
            search_term: "MyApp.Accounts".to_string(),
            kind_filter: None,
            project: "default".to_string(),
            total_items: 3,
            definitions: vec![
                Definition::Type {
                    module: "MyApp.Accounts".to_string(),
                    name: "user".to_string(),
                    line: 5,
                    kind: "type".to_string(),
                    params: String::new(),
                    definition: "@type user() :: %{id: integer()}".to_string(),
                },
                Definition::Spec {
                    module: "MyApp.Accounts".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                    line: 8,
                    kind: "spec".to_string(),
                    inputs: "integer()".to_string(),
                    returns: "User.t()".to_string(),
                    full: "@spec get_user(integer()) :: User.t()".to_string(),
                },
                Definition::Function {
                    module: "MyApp.Accounts".to_string(),
                    file: "lib/accounts.ex".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                    line: 10,
                    start_line: 10,
                    end_line: 15,
                    kind: "def".to_string(),
                    args: String::new(),
                    return_type: String::new(),
                    pattern: String::new(),
                    guard: String::new(),
                },
            ],
        }
    }

    #[fixture]
    fn struct_result() -> BrowseModuleResult {
        BrowseModuleResult {
            search_term: "MyApp.User".to_string(),
            kind_filter: Some(DefinitionKind::Structs),
            project: "default".to_string(),
            total_items: 1,
            definitions: vec![Definition::Struct {
                module: "MyApp.User".to_string(),
                name: "MyApp.User".to_string(),
                fields: vec![
                    FieldInfo {
                        name: "id".to_string(),
                        inferred_type: "integer()".to_string(),
                        default_value: "nil".to_string(),
                        required: true,
                    },
                    FieldInfo {
                        name: "name".to_string(),
                        inferred_type: "String.t()".to_string(),
                        default_value: "nil".to_string(),
                        required: false,
                    },
                ],
            }],
        }
    }

    // =========================================================================
    // Table format tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: BrowseModuleResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_functions_only,
        fixture: functions_only_result,
        fixture_type: BrowseModuleResult,
        expected: FUNCTIONS_ONLY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_mixed_types,
        fixture: mixed_types_result,
        fixture_type: BrowseModuleResult,
        expected: MIXED_TYPES_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_struct,
        fixture: struct_result,
        fixture_type: BrowseModuleResult,
        expected: STRUCT_TABLE,
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    #[rstest]
    fn test_json_format_contains_type_discriminant(functions_only_result: BrowseModuleResult) {
        use crate::output::{OutputFormat, Outputable};

        let json = functions_only_result.format(OutputFormat::Json);

        // Verify the type tag is present for each definition
        assert!(json.contains("\"type\": \"function\""));
    }

    #[rstest]
    fn test_json_format_struct_contains_fields(struct_result: BrowseModuleResult) {
        use crate::output::{OutputFormat, Outputable};

        let json = struct_result.format(OutputFormat::Json);

        assert!(json.contains("\"type\": \"struct\""));
        assert!(json.contains("\"fields\""));
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"name\""));
    }

    // =========================================================================
    // Toon format tests
    // =========================================================================

    #[rstest]
    fn test_toon_format_compact(functions_only_result: BrowseModuleResult) {
        use crate::output::{OutputFormat, Outputable};

        let toon = functions_only_result.format(OutputFormat::Toon);

        // Toon format should be more compact than JSON
        let json = functions_only_result.format(OutputFormat::Json);
        assert!(toon.len() < json.len(), "Toon should be more compact than JSON");

        // Should contain key information
        assert!(toon.contains("MyApp.Accounts"));
        assert!(toon.contains("get_user"));
    }
}
