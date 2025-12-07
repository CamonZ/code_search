//! Output formatting for file command results.

use crate::output::Outputable;
use super::execute::FileResult;

impl Outputable for FileResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Functions in files matching '{}' (project: {})", self.file_pattern, self.project));
        lines.push(String::new());

        if !self.files.is_empty() {
            for file_info in &self.files {
                lines.push(format!("{}:", file_info.file));
                for func in &file_info.functions {
                    let sig = format!("{}.{}/{}", func.module, func.name, func.arity);
                    lines.push(format!(
                        "  {:>4}-{:<4} [{}] {}",
                        func.start_line, func.end_line, func.kind, sig
                    ));
                }
                lines.push(String::new());
            }
            // Remove trailing empty line
            if lines.last() == Some(&String::new()) {
                lines.pop();
            }
        } else {
            lines.push("No files found.".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::{FileWithFunctions, FileFunctionDef};
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    const EMPTY_TABLE_OUTPUT: &str = "\
Functions in files matching 'nonexistent.ex' (project: test_project)

No files found.";

    const SINGLE_FILE_OUTPUT: &str = "\
Functions in files matching 'lib/accounts.ex' (project: test_project)

lib/accounts.ex:
    10-20   [def] MyApp.Accounts.get_user/1
    25-30   [def] MyApp.Accounts.list_users/0";

    const MULTIPLE_FILES_OUTPUT: &str = "\
Functions in files matching 'lib/' (project: test_project)

lib/accounts.ex:
    10-20   [def] MyApp.Accounts.get_user/1

lib/service.ex:
     5-15   [def] MyApp.Service.process/1";

    #[fixture]
    fn empty_result() -> FileResult {
        FileResult {
            project: "test_project".to_string(),
            file_pattern: "nonexistent.ex".to_string(),
            files: vec![],
        }
    }

    #[fixture]
    fn single_file_result() -> FileResult {
        FileResult {
            project: "test_project".to_string(),
            file_pattern: "lib/accounts.ex".to_string(),
            files: vec![FileWithFunctions {
                file: "lib/accounts.ex".to_string(),
                functions: vec![
                    FileFunctionDef {
                        module: "MyApp.Accounts".to_string(),
                        name: "get_user".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        start_line: 10,
                        end_line: 20,
                    },
                    FileFunctionDef {
                        module: "MyApp.Accounts".to_string(),
                        name: "list_users".to_string(),
                        arity: 0,
                        kind: "def".to_string(),
                        start_line: 25,
                        end_line: 30,
                    },
                ],
            }],
        }
    }

    #[fixture]
    fn multiple_files_result() -> FileResult {
        FileResult {
            project: "test_project".to_string(),
            file_pattern: "lib/".to_string(),
            files: vec![
                FileWithFunctions {
                    file: "lib/accounts.ex".to_string(),
                    functions: vec![FileFunctionDef {
                        module: "MyApp.Accounts".to_string(),
                        name: "get_user".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        start_line: 10,
                        end_line: 20,
                    }],
                },
                FileWithFunctions {
                    file: "lib/service.ex".to_string(),
                    functions: vec![FileFunctionDef {
                        module: "MyApp.Service".to_string(),
                        name: "process".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        start_line: 5,
                        end_line: 15,
                    }],
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: FileResult) {
        assert_eq!(empty_result.to_table(), EMPTY_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_single_file(single_file_result: FileResult) {
        assert_eq!(single_file_result.to_table(), SINGLE_FILE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_multiple_files(multiple_files_result: FileResult) {
        assert_eq!(multiple_files_result.to_table(), MULTIPLE_FILES_OUTPUT);
    }

    #[rstest]
    fn test_format_json(single_file_result: FileResult) {
        let output = single_file_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["project"], "test_project");
        assert_eq!(parsed["file_pattern"], "lib/accounts.ex");
        assert_eq!(parsed["files"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["files"][0]["functions"].as_array().unwrap().len(), 2);
    }

    #[rstest]
    fn test_format_toon(single_file_result: FileResult) {
        let output = single_file_result.format(OutputFormat::Toon);
        assert!(output.contains("project: test_project"));
        assert!(output.contains("file_pattern: lib/accounts.ex"));
    }

    #[rstest]
    fn test_format_toon_file_fields(single_file_result: FileResult) {
        let output = single_file_result.format(OutputFormat::Toon);
        // Toon renders nested structures - check for files array
        assert!(output.contains("files[1]"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: FileResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("project: test_project"));
        assert!(output.contains("files[0]"));
    }
}
