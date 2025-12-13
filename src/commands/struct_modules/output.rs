//! Output formatting for struct-modules command results.

use crate::output::Outputable;
use super::execute::StructModulesResult;

impl Outputable for StructModulesResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push(format!("Modules using \"{}\"", self.struct_pattern));
        lines.push(String::new());

        if self.modules.is_empty() {
            lines.push("No modules found.".to_string());
            return lines.join("\n");
        }

        // Summary
        lines.push(format!(
            "Found {} module(s) ({} function(s)):",
            self.total_modules, self.total_functions
        ));
        lines.push(String::new());

        // Table header
        lines.push("Module                      Accepts  Returns  Total".to_string());
        lines.push("──────────────────────────────────────────────────".to_string());

        // Table rows
        for module in &self.modules {
            let line = format!(
                "{:<28} {:>7} {:>8} {:>5}",
                truncate_module_name(&module.name, 28),
                module.accepts_count,
                module.returns_count,
                module.total
            );
            lines.push(line);
        }

        lines.join("\n")
    }
}

/// Truncate module name to max width with ellipsis if needed
fn truncate_module_name(name: &str, max_width: usize) -> String {
    if name.len() > max_width {
        format!("{}…", &name[..max_width - 1])
    } else {
        name.to_string()
    }
}
