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
                lines.push(format!(
                    "\n  [{}] {}",
                    struct_def.project, struct_def.module
                ));
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

    fn to_terse(&self) -> String {
        if self.structs.is_empty() {
            String::new()
        } else {
            self.structs
                .iter()
                .flat_map(|s| {
                    s.fields.iter().map(move |f| {
                        format!(
                            "{},{},{},{},{},{}",
                            s.project, s.module, f.name, f.default_value, f.required, f.inferred_type
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::{StructDefinition, FieldInfo};
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    #[fixture]
    fn empty_result() -> StructResult {
        StructResult {
            module_pattern: "MyApp.User".to_string(),
            structs: vec![],
        }
    }

    #[fixture]
    fn single_result() -> StructResult {
        StructResult {
            module_pattern: "MyApp.User".to_string(),
            structs: vec![StructDefinition {
                project: "default".to_string(),
                module: "MyApp.User".to_string(),
                fields: vec![
                    FieldInfo {
                        name: "id".to_string(),
                        default_value: "nil".to_string(),
                        required: true,
                        inferred_type: "integer()".to_string(),
                    },
                    FieldInfo {
                        name: "name".to_string(),
                        default_value: "nil".to_string(),
                        required: false,
                        inferred_type: "String.t()".to_string(),
                    },
                ],
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> StructResult {
        StructResult {
            module_pattern: "MyApp.*".to_string(),
            structs: vec![
                StructDefinition {
                    project: "default".to_string(),
                    module: "MyApp.User".to_string(),
                    fields: vec![FieldInfo {
                        name: "id".to_string(),
                        default_value: "nil".to_string(),
                        required: true,
                        inferred_type: "integer()".to_string(),
                    }],
                },
                StructDefinition {
                    project: "default".to_string(),
                    module: "MyApp.Post".to_string(),
                    fields: vec![FieldInfo {
                        name: "title".to_string(),
                        default_value: "nil".to_string(),
                        required: true,
                        inferred_type: "String.t()".to_string(),
                    }],
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: StructResult) {
        let output = empty_result.to_table();
        assert!(output.contains("Struct: MyApp.User"));
        assert!(output.contains("No structs found."));
    }

    #[rstest]
    fn test_to_table_single(single_result: StructResult) {
        let output = single_result.to_table();
        assert!(output.contains("Struct: MyApp.User"));
        assert!(output.contains("Found 1 struct(s):"));
        assert!(output.contains("[default] MyApp.User"));
        assert!(output.contains("*id :: integer() \\ nil"));
        assert!(output.contains("name :: String.t() \\ nil"));
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: StructResult) {
        let output = multiple_result.to_table();
        assert!(output.contains("Found 2 struct(s):"));
        assert!(output.contains("MyApp.User"));
        assert!(output.contains("MyApp.Post"));
    }

    #[rstest]
    fn test_to_terse_empty(empty_result: StructResult) {
        assert_eq!(empty_result.to_terse(), "");
    }

    #[rstest]
    fn test_to_terse_single(single_result: StructResult) {
        let output = single_result.to_terse();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "default,MyApp.User,id,nil,true,integer()");
        assert_eq!(lines[1], "default,MyApp.User,name,nil,false,String.t()");
    }

    #[rstest]
    fn test_format_json(single_result: StructResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["module_pattern"], "MyApp.User");
        assert_eq!(parsed["structs"].as_array().unwrap().len(), 1);
    }

    #[rstest]
    fn test_format_toon(single_result: StructResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("module_pattern: MyApp.User"));
    }

    #[rstest]
    fn test_format_toon_struct_fields(single_result: StructResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("structs[1]"));
        assert!(output.contains("fields[2]"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: StructResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("structs[0]"));
    }
}
