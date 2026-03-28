//! Output formatting tests for cycles command.

#[cfg(test)]
mod tests {
    use super::super::execute::{Cycle, CyclesResult};
    use crate::output::{OutputFormat, Outputable};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "No circular dependencies found.\n";

    const SINGLE_CYCLE_TABLE: &str = "\
Circular Dependencies

Found 1 cycle(s):

Cycle 1 (length 2):
  MyApp.Accounts
    → MyApp.Auth
    → MyApp.Accounts

Total: 2 module(s) involved in cycles
";

    const MULTIPLE_CYCLES_TABLE: &str = "\
Circular Dependencies

Found 2 cycle(s):

Cycle 1 (length 2):
  A
    → B
    → A

Cycle 2 (length 3):
  C
    → D
    → E
    → C

Total: 5 module(s) involved in cycles
";

    // =========================================================================
    // Fixtures
    // =========================================================================

    #[fixture]
    fn empty_result() -> CyclesResult {
        CyclesResult {
            total_cycles: 0,
            modules_in_cycles: 0,
            cycles: vec![],
        }
    }

    #[fixture]
    fn single_cycle_result() -> CyclesResult {
        CyclesResult {
            total_cycles: 1,
            modules_in_cycles: 2,
            cycles: vec![Cycle {
                length: 2,
                modules: vec!["MyApp.Accounts".to_string(), "MyApp.Auth".to_string()],
            }],
        }
    }

    #[fixture]
    fn multiple_cycles_result() -> CyclesResult {
        CyclesResult {
            total_cycles: 2,
            modules_in_cycles: 5,
            cycles: vec![
                Cycle {
                    length: 2,
                    modules: vec!["A".to_string(), "B".to_string()],
                },
                Cycle {
                    length: 3,
                    modules: vec!["C".to_string(), "D".to_string(), "E".to_string()],
                },
            ],
        }
    }

    // =========================================================================
    // Table format snapshot tests
    // =========================================================================

    #[rstest]
    fn test_to_table_empty(empty_result: CyclesResult) {
        let output = empty_result.to_table();
        assert_eq!(output, EMPTY_TABLE);
    }

    #[rstest]
    fn test_to_table_single_cycle(single_cycle_result: CyclesResult) {
        let output = single_cycle_result.to_table();
        assert_eq!(output, SINGLE_CYCLE_TABLE);
    }

    #[rstest]
    fn test_to_table_multiple_cycles(multiple_cycles_result: CyclesResult) {
        let output = multiple_cycles_result.to_table();
        assert_eq!(output, MULTIPLE_CYCLES_TABLE);
    }

    // =========================================================================
    // Specific mutant-killing tests
    // =========================================================================

    /// Tests that the first module in a cycle gets "  " prefix (not arrow prefix).
    /// Kills: output.rs:21 == replaced with != (would give first module arrow prefix
    /// and all other modules the "  " prefix instead)
    #[rstest]
    fn test_first_module_gets_indent_not_arrow(single_cycle_result: CyclesResult) {
        let output = single_cycle_result.to_table();

        // The first module after "Cycle N (length M):" should start with "  " (2-space indent)
        // not with "→"
        let lines: Vec<&str> = output.lines().collect();
        // Find the line after "Cycle 1 (length 2):"
        let cycle_header_idx = lines
            .iter()
            .position(|l| l.contains("Cycle 1"))
            .expect("Should have cycle header");
        let first_module_line = lines[cycle_header_idx + 1];
        assert!(
            first_module_line.starts_with("  ") && !first_module_line.contains("→"),
            "First module should start with indent, not arrow. Got: '{}'",
            first_module_line
        );

        // The second module should have the arrow prefix
        let second_module_line = lines[cycle_header_idx + 2];
        assert!(
            second_module_line.contains("→"),
            "Second module should have arrow prefix. Got: '{}'",
            second_module_line
        );
    }

    /// Tests that non-empty cycles get a closing arrow back to the first module.
    /// Kills: output.rs:30 delete ! (if !cycle.modules.is_empty() becomes
    /// if cycle.modules.is_empty(), so closing arrow would only show for empty cycles)
    #[rstest]
    fn test_closing_arrow_back_to_first_module(single_cycle_result: CyclesResult) {
        let output = single_cycle_result.to_table();

        // The cycle path should end with "→ <first_module>" to close the loop
        // For the single cycle with modules [MyApp.Accounts, MyApp.Auth]:
        // We expect:
        //   MyApp.Accounts
        //     → MyApp.Auth
        //     → MyApp.Accounts   <-- closing arrow
        let lines: Vec<&str> = output.lines().collect();
        let cycle_header_idx = lines
            .iter()
            .position(|l| l.contains("Cycle 1"))
            .expect("Should have cycle header");

        // The closing arrow line should reference the first module
        let closing_line = lines[cycle_header_idx + 3]; // first module, second module, then closing
        assert!(
            closing_line.contains("→") && closing_line.contains("MyApp.Accounts"),
            "Should have closing arrow back to first module. Got: '{}'",
            closing_line
        );
    }

    /// Tests that a cycle with a single module (length 1 conceptually) still gets
    /// the closing arrow if modules is non-empty.
    #[test]
    fn test_closing_arrow_with_single_module_cycle() {
        let result = CyclesResult {
            total_cycles: 1,
            modules_in_cycles: 1,
            cycles: vec![Cycle {
                length: 1,
                modules: vec!["OnlyModule".to_string()],
            }],
        };

        let output = result.to_table();

        // Should contain the closing arrow back to OnlyModule
        // The output should have:
        //   OnlyModule
        //     → OnlyModule
        let arrow_count = output.matches("→ OnlyModule").count();
        assert_eq!(
            arrow_count, 1,
            "Should have exactly one closing arrow to OnlyModule. Output:\n{}",
            output
        );
    }

    /// Tests that an empty modules vec does NOT produce a closing arrow.
    /// This is the inverse case for the ! deletion mutant.
    #[test]
    fn test_no_closing_arrow_for_empty_modules() {
        let result = CyclesResult {
            total_cycles: 1,
            modules_in_cycles: 0,
            cycles: vec![Cycle {
                length: 0,
                modules: vec![],
            }],
        };

        let output = result.to_table();

        // With empty modules, there should be no arrow at all
        assert!(
            !output.contains("→"),
            "Empty modules cycle should not have closing arrow. Got: {}",
            output
        );
    }

    // =========================================================================
    // to_table return value tests (kill to_table -> String::new() and "xyzzy")
    // =========================================================================

    #[rstest]
    fn test_to_table_not_empty_string(single_cycle_result: CyclesResult) {
        let output = single_cycle_result.to_table();
        assert!(!output.is_empty(), "to_table should not return empty string");
        assert_ne!(output, "xyzzy", "to_table should not return 'xyzzy'");
        assert!(
            output.contains("Circular Dependencies"),
            "to_table should contain header"
        );
    }

    #[rstest]
    fn test_to_table_empty_not_xyzzy(empty_result: CyclesResult) {
        let output = empty_result.to_table();
        assert!(!output.is_empty(), "Even empty result should produce output");
        assert_ne!(output, "xyzzy", "to_table should not return 'xyzzy'");
        assert!(
            output.contains("No circular dependencies found"),
            "Empty result should show no-cycles message"
        );
    }

    // =========================================================================
    // JSON format tests
    // =========================================================================

    #[rstest]
    fn test_format_json(single_cycle_result: CyclesResult) {
        let output = single_cycle_result.format(OutputFormat::Json);
        assert!(output.contains("\"total_cycles\": 1"));
        assert!(output.contains("\"modules_in_cycles\": 2"));
        assert!(output.contains("MyApp.Accounts"));
        assert!(output.contains("MyApp.Auth"));
    }

    #[rstest]
    fn test_format_json_empty(empty_result: CyclesResult) {
        let output = empty_result.format(OutputFormat::Json);
        assert!(output.contains("\"total_cycles\": 0"));
        assert!(output.contains("\"modules_in_cycles\": 0"));
        assert!(output.contains("\"cycles\": []"));
    }

    // =========================================================================
    // Toon format tests
    // =========================================================================

    #[rstest]
    fn test_format_toon(single_cycle_result: CyclesResult) {
        let output = single_cycle_result.format(OutputFormat::Toon);
        assert!(output.contains("total_cycles"));
        assert!(output.contains("modules_in_cycles"));
        assert!(output.contains("cycles"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: CyclesResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("total_cycles"));
        assert!(output.contains("cycles"));
    }

    // =========================================================================
    // Summary line tests
    // =========================================================================

    #[rstest]
    fn test_summary_shows_module_count(multiple_cycles_result: CyclesResult) {
        let output = multiple_cycles_result.to_table();
        assert!(
            output.contains("Total: 5 module(s) involved in cycles"),
            "Should show correct module count in summary"
        );
    }
}
