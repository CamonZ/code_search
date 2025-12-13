//! Output formatting tests for complexity command.

#[cfg(test)]
mod tests {
    use super::super::execute::ComplexityEntry;
    use crate::output::Outputable;
    use crate::types::{ModuleCollectionResult, ModuleGroup};

    #[test]
    fn test_format_table_single_function() {
        let result = ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                entries: vec![ComplexityEntry {
                    name: "create_user".to_string(),
                    arity: 1,
                    line: 10,
                    complexity: 12,
                    max_nesting_depth: 4,
                    lines: 45,
                }],
                function_count: None,
            }],
        };

        let output = result.format(crate::output::OutputFormat::Table);
        assert!(output.contains("Complexity"));
        assert!(output.contains("MyApp.Accounts"));
        assert!(output.contains("create_user/1"));
        assert!(output.contains("complexity: 12"));
        assert!(output.contains("depth: 4"));
        assert!(output.contains("lines: 45"));
    }

    #[test]
    fn test_format_table_empty() {
        let result: ModuleCollectionResult<ComplexityEntry> = ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 0,
            items: vec![],
        };

        let output = result.format(crate::output::OutputFormat::Table);
        assert!(output.contains("Complexity"));
        assert!(output.contains("No functions found"));
    }

    #[test]
    fn test_format_json() {
        let result = ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Accounts".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                entries: vec![ComplexityEntry {
                    name: "create_user".to_string(),
                    arity: 1,
                    line: 10,
                    complexity: 12,
                    max_nesting_depth: 4,
                    lines: 45,
                }],
                function_count: None,
            }],
        };

        let output = result.format(crate::output::OutputFormat::Json);
        // Verify it's valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Output should be valid JSON");
        assert_eq!(
            parsed["total_items"], 1,
            "total_items should be 1"
        );
        assert_eq!(
            parsed["items"][0]["entries"][0]["complexity"], 12,
            "complexity should be 12"
        );
    }

    #[test]
    fn test_format_toon() {
        let result = ModuleCollectionResult {
            module_pattern: "*".to_string(),
            function_pattern: None,
            kind_filter: None,
            name_filter: None,
            total_items: 1,
            items: vec![ModuleGroup {
                name: "MyApp.Service".to_string(),
                file: "lib/my_app/service.ex".to_string(),
                entries: vec![ComplexityEntry {
                    name: "process".to_string(),
                    arity: 1,
                    line: 5,
                    complexity: 8,
                    max_nesting_depth: 3,
                    lines: 25,
                }],
                function_count: None,
            }],
        };

        let output = result.format(crate::output::OutputFormat::Toon);
        // Verify it contains expected toon output elements
        assert!(output.contains("MyApp.Service"));
        assert!(output.contains("process"));
        assert!(output.contains("8")); // complexity
    }
}
