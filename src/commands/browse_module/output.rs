use std::collections::BTreeMap;

use super::execute::{BrowseModuleResult, Definition};
use crate::output::Outputable;

impl Outputable for BrowseModuleResult {
    fn to_table(&self) -> String {
        let mut output = String::new();

        // Header
        if let Some(kind) = self.kind_filter {
            output.push_str(&format!(
                "Definitions in {} (kind: {}, project: {})\n\n",
                self.search_term, kind, self.project
            ));
        } else {
            output.push_str(&format!(
                "Definitions in {} (project: {})\n\n",
                self.search_term, self.project
            ));
        }

        // Empty state
        if self.definitions.is_empty() {
            output.push_str("No definitions found.\n");
            return output;
        }

        // Summary
        output.push_str(&format!(
            "Found {} definition(s):\n\n",
            self.total_items
        ));

        // Group by module for readability
        let mut by_module: BTreeMap<String, Vec<&Definition>> = BTreeMap::new();
        for def in &self.definitions {
            by_module
                .entry(def.module().to_string())
                .or_insert_with(Vec::new)
                .push(def);
        }

        // Format each module and its definitions
        for (module, definitions) in by_module {
            output.push_str(&format!("  {}:\n", module));

            for def in definitions {
                match def {
                    Definition::Function {
                        name,
                        arity,
                        line,
                        kind,
                        args,
                        return_type,
                        ..
                    } => {
                        output.push_str(&format!("    L{:<3}  [{}] {}/{}\n", line, kind, name, arity));
                        if !args.is_empty() || !return_type.is_empty() {
                            output.push_str(&format!("           {} {}\n", args, return_type).trim_end());
                            output.push('\n');
                        }
                    }

                    Definition::Spec {
                        name,
                        arity,
                        line,
                        kind,
                        full,
                        ..
                    } => {
                        output.push_str(&format!("    L{:<3}  [{}] {}/{}\n", line, kind, name, arity));
                        if !full.is_empty() {
                            output.push_str(&format!("           {}\n", full));
                        }
                    }

                    Definition::Type {
                        name,
                        line,
                        kind,
                        definition,
                        ..
                    } => {
                        output.push_str(&format!("    L{:<3}  [{}] {}\n", line, kind, name));
                        if !definition.is_empty() {
                            output.push_str(&format!("           {}\n", definition));
                        }
                    }

                    Definition::Struct { name, fields, .. } => {
                        output.push_str(&format!("    L0    [struct] {} with {} fields\n", name, fields.len()));
                        for (i, field) in fields.iter().enumerate() {
                            if i >= 5 {
                                output.push_str(&format!("           ... and {} more fields\n", fields.len() - 5));
                                break;
                            }
                            output.push_str(&format!(
                                "           - {}: {} {}\n",
                                field.name,
                                field.inferred_type,
                                if field.required { "(required)" } else { "(optional)" }
                            ));
                        }
                    }
                }
            }

            output.push('\n');
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_result() {
        let result = BrowseModuleResult {
            search_term: "NonExistent".to_string(),
            kind_filter: None,
            project: "default".to_string(),
            total_items: 0,
            definitions: vec![],
        };

        let table = result.to_table();
        assert!(table.contains("No definitions found"));
    }

    #[test]
    fn test_function_formatting() {
        use super::super::execute::Definition;

        let result = BrowseModuleResult {
            search_term: "MyApp.Accounts".to_string(),
            kind_filter: None,
            project: "default".to_string(),
            total_items: 1,
            definitions: vec![Definition::Function {
                module: "MyApp.Accounts".to_string(),
                file: "lib/accounts.ex".to_string(),
                name: "get_user".to_string(),
                arity: 1,
                line: 10,
                start_line: 10,
                end_line: 20,
                kind: "def".to_string(),
                args: "(integer())".to_string(),
                return_type: "User.t() | nil".to_string(),
                pattern: String::new(),
                guard: String::new(),
            }],
        };

        let table = result.to_table();
        assert!(table.contains("MyApp.Accounts"));
        assert!(table.contains("get_user/1"));
        assert!(table.contains("[def]"));
        assert!(table.contains("L10"));
    }

    #[test]
    fn test_mixed_types_formatting() {
        use super::super::execute::Definition;

        let result = BrowseModuleResult {
            search_term: "MyApp.Accounts".to_string(),
            kind_filter: None,
            project: "default".to_string(),
            total_items: 2,
            definitions: vec![
                Definition::Function {
                    module: "MyApp.Accounts".to_string(),
                    file: "lib/accounts.ex".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                    line: 10,
                    start_line: 10,
                    end_line: 20,
                    kind: "def".to_string(),
                    args: String::new(),
                    return_type: String::new(),
                    pattern: String::new(),
                    guard: String::new(),
                },
                Definition::Type {
                    module: "MyApp.Accounts".to_string(),
                    name: "user".to_string(),
                    line: 5,
                    kind: "type".to_string(),
                    params: String::new(),
                    definition: "@type user() :: %{}".to_string(),
                },
            ],
        };

        let table = result.to_table();
        assert!(table.contains("MyApp.Accounts"));
        assert!(table.contains("[def]"));
        assert!(table.contains("[type]"));
        assert!(table.contains("Found 2 definition(s)"));
    }
}
