//! Output formatting tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::execute::{FunctionHotspotEntry, FunctionHotspotsResult, ModuleCountEntry, ModuleHotspotsResult, HotspotsResult};
    use crate::output::Outputable;
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs for function-level hotspots
    // =========================================================================

    const FUNCTION_EMPTY_TABLE: &str = "\
Hotspots (total)

No hotspots found.";

    const FUNCTION_SINGLE_TABLE: &str = "\
Hotspots (total)

Found 1 function:

MyApp.Accounts.get_user    in: 3  out: 1  total: 4  ratio: 0.25";

    // =========================================================================
    // Expected outputs for module-level hotspots
    // =========================================================================

    const MODULE_SINGLE_TABLE: &str = "\
Hotspots (functions)

Found 1 module:

MyApp.Accounts                              27 functions";

    // =========================================================================
    // Fixtures for function-level hotspots
    // =========================================================================

    #[fixture]
    fn function_empty_result() -> FunctionHotspotsResult {
        FunctionHotspotsResult {
            kind: "total".to_string(),
            module_pattern: "*".to_string(),
            total_items: 0,
            entries: vec![],
        }
    }

    #[fixture]
    fn function_single_result() -> FunctionHotspotsResult {
        FunctionHotspotsResult {
            kind: "total".to_string(),
            module_pattern: "*".to_string(),
            total_items: 1,
            entries: vec![FunctionHotspotEntry {
                module: "MyApp.Accounts".to_string(),
                function: "get_user".to_string(),
                incoming: 3,
                outgoing: 1,
                total: 4,
                ratio: 0.25,
            }],
        }
    }

    // =========================================================================
    // Fixtures for module-level hotspots
    // =========================================================================

    #[fixture]
    fn module_single_result() -> ModuleHotspotsResult {
        ModuleHotspotsResult {
            kind: "functions".to_string(),
            module_pattern: "*".to_string(),
            total_items: 1,
            entries: vec![ModuleCountEntry {
                module: "MyApp.Accounts".to_string(),
                count: 27,
            }],
        }
    }

    // =========================================================================
    // Tests for function-level hotspots
    // =========================================================================

    #[rstest]
    fn test_function_to_table_empty(function_empty_result: FunctionHotspotsResult) {
        let output = function_empty_result.to_table();
        assert_eq!(output, FUNCTION_EMPTY_TABLE);
    }

    #[rstest]
    fn test_function_to_table_single(function_single_result: FunctionHotspotsResult) {
        let output = function_single_result.to_table();
        assert_eq!(output, FUNCTION_SINGLE_TABLE);
    }

    #[rstest]
    fn test_function_format_json(function_single_result: FunctionHotspotsResult) {
        let output = function_single_result.format(crate::output::OutputFormat::Json);
        assert!(output.contains("\"kind\": \"total\""));
        assert!(output.contains("\"function\": \"get_user\""));
        assert!(output.contains("\"incoming\": 3"));
    }

    // =========================================================================
    // Tests for module-level hotspots
    // =========================================================================

    #[rstest]
    fn test_module_to_table_single(module_single_result: ModuleHotspotsResult) {
        let output = module_single_result.to_table();
        assert_eq!(output, MODULE_SINGLE_TABLE);
    }

    #[rstest]
    fn test_module_format_json(module_single_result: ModuleHotspotsResult) {
        let output = module_single_result.format(crate::output::OutputFormat::Json);
        assert!(output.contains("\"kind\": \"functions\""));
        assert!(output.contains("\"count\": 27"));
        assert!(output.contains("\"module\": \"MyApp.Accounts\""));
    }

    // =========================================================================
    // Tests for enum wrapper
    // =========================================================================

    #[rstest]
    fn test_enum_function_variant(function_single_result: FunctionHotspotsResult) {
        let result = HotspotsResult::Functions(function_single_result);
        let output = result.to_table();
        assert!(output.contains("Hotspots (total)"));
        assert!(output.contains("MyApp.Accounts.get_user"));
    }

    #[rstest]
    fn test_enum_module_variant(module_single_result: ModuleHotspotsResult) {
        let result = HotspotsResult::Modules(module_single_result);
        let output = result.to_table();
        assert!(output.contains("Hotspots (functions)"));
        assert!(output.contains("27 functions"));
    }
}
