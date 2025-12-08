//! Output formatting for struct command results.

use crate::output::Outputable;
use super::execute::StructResult;

impl Outputable for StructResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        let header = format!("Struct: {}", self.module_pattern);
        lines.push(header);
        lines.push(String::new());

        if !self.structs.is_empty() {
            lines.push(format!("Found {} struct(s):", self.structs.len()));
            for struct_def in &self.structs {
                lines.push(format!("\n  {}", struct_def.module));
                for field in &struct_def.fields {
                    let required_marker = if field.required { "*" } else { "" };
                    let type_info = if field.inferred_type.is_empty() {
                        String::new()
                    } else {
                        format!(" :: {}", field.inferred_type)
                    };
                    let default_info = if field.default_value.is_empty() {
                        String::new()
                    } else {
                        format!(" \\ {}", field.default_value)
                    };
                    lines.push(format!(
                        "    {}{name}{type_info}{default_info}",
                        required_marker,
                        name = field.name,
                    ));
                }
            }
        } else {
            lines.push("No structs found.".to_string());
        }

        lines.join("\n")
    }
}
