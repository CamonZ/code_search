use crate::output::Outputable;

use super::execute::{DuplicatesByModuleResult, DuplicatesOutput, DuplicatesResult};

impl Outputable for DuplicatesResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push("Duplicate Functions".to_string());
        lines.push(String::new());

        if self.groups.is_empty() {
            lines.push("No duplicate functions found.".to_string());
        } else {
            lines.push(format!(
                "Found {} group(s) of duplicate(s) ({} function(s) total):",
                self.total_groups, self.total_duplicates
            ));
            lines.push(String::new());

            for (idx, group) in self.groups.iter().enumerate() {
                // Format hash - truncate for readability
                let hash_display = if group.hash.len() > 20 {
                    format!("{}...", &group.hash[..17])
                } else {
                    group.hash.clone()
                };

                lines.push(format!(
                    "Group {} - hash:{}... ({} function(s)):",
                    idx + 1,
                    hash_display,
                    group.functions.len()
                ));

                for func in &group.functions {
                    lines.push(format!(
                        "  {}.{}/{} L{}  {}",
                        func.module, func.name, func.arity, func.line, func.file
                    ));
                }
                lines.push(String::new());
            }
        }

        lines.join("\n")
    }
}

impl Outputable for DuplicatesByModuleResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push("Modules with Most Duplicates".to_string());
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No duplicate functions found.".to_string());
        } else {
            lines.push(format!(
                "Found {} duplicated function(s) across {} module(s):",
                self.total_duplicates, self.total_modules
            ));
            lines.push(String::new());

            for module in &self.modules {
                lines.push(format!("{} ({} duplicates):", module.name, module.duplicate_count));

                for dup in &module.top_duplicates {
                    lines.push(format!(
                        "  {}/{} ({} copies)",
                        dup.name, dup.arity, dup.copy_count
                    ));
                }
                lines.push(String::new());
            }
        }

        lines.join("\n")
    }
}

impl Outputable for DuplicatesOutput {
    fn to_table(&self) -> String {
        match self {
            DuplicatesOutput::Detailed(result) => result.to_table(),
            DuplicatesOutput::ByModule(result) => result.to_table(),
        }
    }
}
