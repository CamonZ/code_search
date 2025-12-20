//! Output formatting for struct-usage command results.

use regex::Regex;
use std::sync::LazyLock;

use crate::output::{Outputable, TableFormatter};
use crate::types::ModuleGroupResult;
use super::execute::{UsageInfo, StructUsageOutput, StructModulesResult};

/// Regex to match Elixir struct maps like `%{__struct__: Module.Name, field: type(), ...}`
static STRUCT_MAP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"%\{__struct__:\s*([A-Za-z][A-Za-z0-9_.]*),\s*[^}]+\}").unwrap()
});

/// Simplify struct representations in type strings.
/// Converts `%{__struct__: Module.Name, field: type(), ...}` to `%Module.Name{}`
fn simplify_structs(s: &str) -> String {
    STRUCT_MAP_REGEX.replace_all(s, "%$1{}").to_string()
}

impl TableFormatter for ModuleGroupResult<UsageInfo> {
    type Entry = UsageInfo;

    fn format_header(&self) -> String {
        let pattern = self.function_pattern.as_ref().map(|s| s.as_str()).unwrap_or("*");
        format!("Functions using \"{}\"", pattern)
    }

    fn format_empty_message(&self) -> String {
        "No functions found.".to_string()
    }

    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} function(s) in {} module(s):", total, module_count)
    }

    fn format_module_header(&self, module_name: &str, _module_file: &str) -> String {
        format!("{}:", module_name)
    }

    fn format_entry(&self, usage_info: &UsageInfo, _module: &str, _file: &str) -> String {
        format!(
            "{}/{} accepts: {} returns: {}",
            usage_info.name,
            usage_info.arity,
            simplify_structs(&usage_info.inputs),
            simplify_structs(&usage_info.returns)
        )
    }
}

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

impl Outputable for StructUsageOutput {
    fn to_table(&self) -> String {
        match self {
            StructUsageOutput::Detailed(result) => result.to_table(),
            StructUsageOutput::ByModule(result) => result.to_table(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplify_structs_basic() {
        let input = "%{__struct__: TradeGym.User, name: binary(), age: integer()}";
        let expected = "%TradeGym.User{}";
        assert_eq!(simplify_structs(input), expected);
    }

    #[test]
    fn test_simplify_structs_with_meta() {
        let input = "%{__struct__: TradeGym.Repo.Schemas.User, __meta__: term(), name: binary()}";
        let expected = "%TradeGym.Repo.Schemas.User{}";
        assert_eq!(simplify_structs(input), expected);
    }

    #[test]
    fn test_simplify_structs_multiple() {
        let input = "%{__struct__: Foo.Bar, x: 1} | %{__struct__: Baz.Qux, y: 2}";
        let expected = "%Foo.Bar{} | %Baz.Qux{}";
        assert_eq!(simplify_structs(input), expected);
    }

    #[test]
    fn test_simplify_structs_no_match() {
        let input = "integer() | binary()";
        assert_eq!(simplify_structs(input), input);
    }
}
