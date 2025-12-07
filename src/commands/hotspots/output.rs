//! Output formatting for hotspots command results.

use crate::output::Outputable;
use super::execute::HotspotsResult;

impl Outputable for HotspotsResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let filter_info = match &self.module_filter {
            Some(pattern) => format!(" (module filter: {})", pattern),
            None => String::new(),
        };
        lines.push(format!("Hotspots ({}) in project '{}'{}", self.kind, self.project, filter_info));
        lines.push(String::new());

        if !self.hotspots.is_empty() {
            // Header
            lines.push(format!("{:<50} {:>8} {:>8} {:>8}", "FUNCTION", "IN", "OUT", "TOTAL"));
            lines.push("-".repeat(78));

            for hotspot in &self.hotspots {
                let sig = format!("{}.{}", hotspot.module, hotspot.function);
                lines.push(format!(
                    "{:<50} {:>8} {:>8} {:>8}",
                    sig, hotspot.incoming, hotspot.outgoing, hotspot.total
                ));
            }
        } else {
            lines.push("No hotspots found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::Hotspot;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    const EMPTY_TABLE_OUTPUT: &str = "\
Hotspots (incoming) in project 'test_project'

No hotspots found.";

    const SINGLE_TABLE_OUTPUT: &str = "\
Hotspots (incoming) in project 'test_project'

FUNCTION                                                 IN      OUT    TOTAL
------------------------------------------------------------------------------
MyApp.Accounts.get_user                                   3        1        4";

    #[fixture]
    fn empty_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "incoming".to_string(),
            module_filter: None,
            hotspots: vec![],
        }
    }

    #[fixture]
    fn single_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "incoming".to_string(),
            module_filter: None,
            hotspots: vec![Hotspot {
                module: "MyApp.Accounts".to_string(),
                function: "get_user".to_string(),
                incoming: 3,
                outgoing: 1,
                total: 4,
            }],
        }
    }

    #[fixture]
    fn filtered_result() -> HotspotsResult {
        HotspotsResult {
            project: "test_project".to_string(),
            kind: "outgoing".to_string(),
            module_filter: Some("Service".to_string()),
            hotspots: vec![Hotspot {
                module: "MyApp.Service".to_string(),
                function: "process".to_string(),
                incoming: 0,
                outgoing: 5,
                total: 5,
            }],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: HotspotsResult) {
        assert_eq!(empty_result.to_table(), EMPTY_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_single(single_result: HotspotsResult) {
        assert_eq!(single_result.to_table(), SINGLE_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_filtered(filtered_result: HotspotsResult) {
        let output = filtered_result.to_table();
        assert!(output.contains("(module filter: Service)"));
        assert!(output.contains("outgoing"));
    }

    #[rstest]
    fn test_format_json(single_result: HotspotsResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["project"], "test_project");
        assert_eq!(parsed["kind"], "incoming");
        assert_eq!(parsed["hotspots"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["hotspots"][0]["incoming"], 3);
    }

    #[rstest]
    fn test_format_toon(single_result: HotspotsResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("project: test_project"));
        assert!(output.contains("kind: incoming"));
    }

    #[rstest]
    fn test_format_toon_hotspot_fields(single_result: HotspotsResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("hotspots[1]{"));
        assert!(output.contains("function"));
        assert!(output.contains("incoming"));
        assert!(output.contains("module"));
        assert!(output.contains("outgoing"));
        assert!(output.contains("total"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: HotspotsResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("project: test_project"));
        assert!(output.contains("hotspots[0]"));
    }
}
