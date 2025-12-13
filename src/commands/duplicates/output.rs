use crate::output::Outputable;

use super::execute::DuplicatesResult;

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
