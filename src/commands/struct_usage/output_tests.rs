//! Output formatting tests for struct-usage command.

#[cfg(test)]
mod tests {
    use super::super::execute::{ModuleStructUsage, StructModulesResult, StructUsageOutput, UsageInfo};
    use crate::types::{ModuleGroup, ModuleGroupResult};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs - Detailed mode
    // =========================================================================

    const EMPTY_DETAILED_TABLE: &str = "\
Functions using \"User.t\"

No functions found.";

    const SINGLE_DETAILED_TABLE: &str = "\
Functions using \"User.t\"

Found 1 function(s) in 1 module(s):

MyApp.Accounts:
  get_user/1 accepts: integer() returns: %User{}";

    // =========================================================================
    // Expected outputs - ByModule mode
    // =========================================================================

    const EMPTY_BY_MODULE_TABLE: &str = "\
Modules using \"User.t\"

No modules found.";

    const SINGLE_BY_MODULE_TABLE: &str = "\
Modules using \"User.t\"

Found 1 module(s) (2 function(s)):

Module                      Accepts  Returns  Total
──────────────────────────────────────────────────
MyApp.Accounts                     1        2     2";

    // =========================================================================
    // Fixtures - Detailed mode
    // =========================================================================

    #[fixture]
    fn empty_detailed() -> StructUsageOutput {
        StructUsageOutput::Detailed(ModuleGroupResult {
            module_pattern: "*".to_string(),
            function_pattern: Some("User.t".to_string()),
            total_items: 0,
            items: vec![],
        })
    }

    #[fixture]
    fn single_detailed() -> StructUsageOutput {
        StructUsageOutput::Detailed(ModuleGroupResult {
            module_pattern: "*".to_string(),
            function_pattern: Some("User.t".to_string()),
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                function_count: Some(1),
                entries: vec![UsageInfo {
                    name: "get_user".to_string(),
                    arity: 1,
                    inputs: "integer()".to_string(),
                    returns: "%{__struct__: User, id: integer()}".to_string(),
                    line: 10,
                }],
            }],
        })
    }

    // =========================================================================
    // Fixtures - ByModule mode
    // =========================================================================

    #[fixture]
    fn empty_by_module() -> StructUsageOutput {
        StructUsageOutput::ByModule(StructModulesResult {
            struct_pattern: "User.t".to_string(),
            total_modules: 0,
            total_functions: 0,
            modules: vec![],
        })
    }

    #[fixture]
    fn single_by_module() -> StructUsageOutput {
        StructUsageOutput::ByModule(StructModulesResult {
            struct_pattern: "User.t".to_string(),
            total_modules: 1,
            total_functions: 2,
            modules: vec![ModuleStructUsage {
                name: "MyApp.Accounts".to_string(),
                accepts_count: 1,
                returns_count: 2,
                total: 2,
            }],
        })
    }

    // =========================================================================
    // Tests - Detailed mode
    // =========================================================================

    crate::output_table_test! {
        test_name: test_detailed_empty,
        fixture: empty_detailed,
        fixture_type: StructUsageOutput,
        expected: EMPTY_DETAILED_TABLE,
    }

    crate::output_table_test! {
        test_name: test_detailed_single,
        fixture: single_detailed,
        fixture_type: StructUsageOutput,
        expected: SINGLE_DETAILED_TABLE,
    }

    // =========================================================================
    // Tests - ByModule mode
    // =========================================================================

    crate::output_table_test! {
        test_name: test_by_module_empty,
        fixture: empty_by_module,
        fixture_type: StructUsageOutput,
        expected: EMPTY_BY_MODULE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_by_module_single,
        fixture: single_by_module,
        fixture_type: StructUsageOutput,
        expected: SINGLE_BY_MODULE_TABLE,
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    #[rstest]
    fn test_detailed_json(single_detailed: StructUsageOutput) {
        use crate::output::{OutputFormat, Outputable};
        let output = single_detailed.format(OutputFormat::Json);
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Should produce valid JSON");

        // Verify structure
        assert!(parsed["items"].is_array());
        assert_eq!(parsed["total_items"], 1);
    }

    #[rstest]
    fn test_by_module_json(single_by_module: StructUsageOutput) {
        use crate::output::{OutputFormat, Outputable};
        let output = single_by_module.format(OutputFormat::Json);
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Should produce valid JSON");

        // Verify structure
        assert!(parsed["modules"].is_array());
        assert_eq!(parsed["total_modules"], 1);
        assert_eq!(parsed["total_functions"], 2);
    }
}
