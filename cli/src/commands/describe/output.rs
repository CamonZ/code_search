//! Output formatting for describe command results.

use crate::output::Outputable;
use super::execute::{DescribeResult, DescribeMode, CategoryListing};
use super::descriptions::CommandDescription;

impl Outputable for DescribeResult {
    fn to_table(&self) -> String {
        match &self.mode {
            DescribeMode::ListAll { categories } => format_list_all(categories),
            DescribeMode::Specific { descriptions } => format_specific(descriptions),
        }
    }
}

fn format_list_all(categories: &[CategoryListing]) -> String {
    let mut output = String::new();
    output.push_str("Available Commands\n");
    output.push('\n');

    for category in categories {
        output.push_str(&format!("{}:\n", category.category));
        for (name, brief) in &category.commands {
            output.push_str(&format!("  {:<20} {}\n", name, brief));
        }
        output.push('\n');
    }

    output.push_str("Use 'code_search describe <command>' for detailed information.\n");
    output
}

fn format_specific(descriptions: &[CommandDescription]) -> String {
    let mut output = String::new();

    for (i, desc) in descriptions.iter().enumerate() {
        if i > 0 {
            output.push('\n');
            output.push_str("================================================================================\n");
            output.push('\n');
        }

        // Title
        output.push_str(&format!("{} - {}\n", desc.name, desc.brief));
        output.push('\n');

        // Description
        output.push_str("DESCRIPTION\n");
        output.push_str(&format!("  {}\n", desc.description));
        output.push('\n');

        // Usage
        output.push_str("USAGE\n");
        output.push_str(&format!("  {}\n", desc.usage));
        output.push('\n');

        // Examples
        if !desc.examples.is_empty() {
            output.push_str("EXAMPLES\n");
            for example in &desc.examples {
                output.push_str(&format!("  # {}\n", example.description));
                output.push_str(&format!("  {}\n", example.command));
                output.push('\n');
            }
        }

        // Related commands
        if !desc.related.is_empty() {
            output.push_str("RELATED COMMANDS\n");
            for related in &desc.related {
                output.push_str(&format!("  {}\n", related));
            }
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::describe::descriptions::Example;

    #[test]
    fn test_format_list_all() {
        let categories = vec![
            CategoryListing {
                category: "Query Commands".to_string(),
                commands: vec![
                    ("calls-to".to_string(), "Find callers of a given function".to_string()),
                    ("calls-from".to_string(), "Find what a function calls".to_string()),
                ],
            },
            CategoryListing {
                category: "Analysis Commands".to_string(),
                commands: vec![
                    ("hotspots".to_string(), "Find high-connectivity functions".to_string()),
                ],
            },
        ];

        let output = format_list_all(&categories);
        assert!(output.contains("Available Commands"));
        assert!(output.contains("Query Commands"));
        assert!(output.contains("Analysis Commands"));
        assert!(output.contains("calls-to"));
        assert!(output.contains("hotspots"));
        assert!(output.contains("Use 'code_search describe <command>' for detailed information."));
    }

    #[test]
    fn test_format_specific_single() {
        let descriptions = vec![
            CommandDescription::new(
                "calls-to",
                "Find callers of a given function",
                crate::commands::describe::descriptions::CommandCategory::Query,
                "Finds all functions that call a specific function.",
                "code_search calls-to -m <MODULE> -f <FUNCTION>",
            )
            .with_examples(vec![
                Example::new("Find all callers", "code_search calls-to -m MyApp.Repo -f get"),
            ])
            .with_related(vec!["calls-from", "trace"]),
        ];

        let output = format_specific(&descriptions);
        assert!(output.contains("calls-to - Find callers of a given function"));
        assert!(output.contains("DESCRIPTION"));
        assert!(output.contains("USAGE"));
        assert!(output.contains("EXAMPLES"));
        assert!(output.contains("Find all callers"));
        assert!(output.contains("RELATED COMMANDS"));
        assert!(output.contains("calls-from"));
    }

    #[test]
    fn test_format_specific_multiple() {
        let descriptions = vec![
            CommandDescription::new(
                "calls-to",
                "Find callers",
                crate::commands::describe::descriptions::CommandCategory::Query,
                "Finds all callers.",
                "code_search calls-to",
            ),
            CommandDescription::new(
                "calls-from",
                "Find callees",
                crate::commands::describe::descriptions::CommandCategory::Query,
                "Finds what is called.",
                "code_search calls-from",
            ),
        ];

        let output = format_specific(&descriptions);
        assert!(output.contains("calls-to"));
        assert!(output.contains("calls-from"));
        assert!(output.contains("================================================================================"));
    }
}
