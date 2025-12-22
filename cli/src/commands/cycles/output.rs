//! Output formatting for cycles command results.

use super::execute::CyclesResult;
use crate::output::Outputable;

impl Outputable for CyclesResult {
    fn to_table(&self) -> String {
        if self.cycles.is_empty() {
            return "No circular dependencies found.\n".to_string();
        }

        let mut output = String::new();
        output.push_str("Circular Dependencies\n\n");
        output.push_str(&format!("Found {} cycle(s):\n\n", self.total_cycles));

        for (idx, cycle) in self.cycles.iter().enumerate() {
            output.push_str(&format!("Cycle {} (length {}):\n", idx + 1, cycle.length));

            // Format the cycle path with arrows
            for (i, module) in cycle.modules.iter().enumerate() {
                if i == 0 {
                    output.push_str("  ");
                } else {
                    output.push_str("\n    → ");
                }
                output.push_str(module);
            }

            // Show closing arrow back to first module
            if !cycle.modules.is_empty() {
                output.push_str("\n    → ");
                output.push_str(&cycle.modules[0]);
            }

            output.push_str("\n\n");
        }

        output.push_str(&format!(
            "Total: {} module(s) involved in cycles\n",
            self.modules_in_cycles
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::cycles::execute::Cycle;

    #[test]
    fn test_cycles_output_format_empty() {
        let result = CyclesResult {
            total_cycles: 0,
            modules_in_cycles: 0,
            cycles: vec![],
        };

        let output = result.to_table();
        assert!(output.contains("No circular dependencies found"));
    }

    #[test]
    fn test_cycles_output_format_single_cycle() {
        let result = CyclesResult {
            total_cycles: 1,
            modules_in_cycles: 2,
            cycles: vec![Cycle {
                length: 2,
                modules: vec!["MyApp.Accounts".to_string(), "MyApp.Auth".to_string()],
            }],
        };

        let output = result.to_table();
        assert!(output.contains("Cycle 1 (length 2)"));
        assert!(output.contains("MyApp.Accounts"));
        assert!(output.contains("MyApp.Auth"));
        assert!(output.contains("Total: 2 module(s) involved in cycles"));
    }

    #[test]
    fn test_cycles_output_format_multiple_cycles() {
        let result = CyclesResult {
            total_cycles: 2,
            modules_in_cycles: 5,
            cycles: vec![
                Cycle {
                    length: 2,
                    modules: vec!["A".to_string(), "B".to_string()],
                },
                Cycle {
                    length: 3,
                    modules: vec![
                        "C".to_string(),
                        "D".to_string(),
                        "E".to_string(),
                    ],
                },
            ],
        };

        let output = result.to_table();
        assert!(output.contains("Cycle 1 (length 2)"));
        assert!(output.contains("Cycle 2 (length 3)"));
        assert!(output.contains("Total: 5 module(s) involved in cycles"));
    }

    #[test]
    fn test_cycles_output_json() {
        let result = CyclesResult {
            total_cycles: 1,
            modules_in_cycles: 2,
            cycles: vec![Cycle {
                length: 2,
                modules: vec!["A".to_string(), "B".to_string()],
            }],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("total_cycles"));
        assert!(json.contains("modules_in_cycles"));
        assert!(json.contains("cycles"));
    }
}
