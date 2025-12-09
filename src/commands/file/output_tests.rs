//! Output formatting tests for file command.

#[cfg(test)]
mod tests {
    use super::super::execute::FileResult;
    use crate::queries::file::{FileFunctionDef, FileWithFunctions};
    use rstest::{fixture, rstest};

    // =========================================================================
    // Expected outputs
    // =========================================================================

    const EMPTY_TABLE: &str = "\
Functions in files matching 'nonexistent.ex' (project: test_project)

No files found.";

    const SINGLE_FILE_TABLE: &str = "\
Functions in files matching 'lib/accounts.ex' (project: test_project)

lib/accounts.ex:
    10-20   [def] MyApp.Accounts.get_user/1
    25-30   [def] MyApp.Accounts.list_users/0";

    const MULTIPLE_FILES_TABLE: &str = "\
Functions in files matching 'lib/' (project: test_project)

lib/accounts.ex:
    10-20   [def] MyApp.Accounts.get_user/1

lib/service.ex:
     5-15   [def] MyApp.Service.process/1";


    // =========================================================================
    // Fixtures
    // =========================================================================

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
                        line: 10,
                        start_line: 10,
                        end_line: 20,
                        pattern: String::new(),
                        guard: String::new(),
                    },
                    FileFunctionDef {
                        module: "MyApp.Accounts".to_string(),
                        name: "list_users".to_string(),
                        arity: 0,
                        kind: "def".to_string(),
                        line: 25,
                        start_line: 25,
                        end_line: 30,
                        pattern: String::new(),
                        guard: String::new(),
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
                        line: 10,
                        start_line: 10,
                        end_line: 20,
                        pattern: String::new(),
                        guard: String::new(),
                    }],
                },
                FileWithFunctions {
                    file: "lib/service.ex".to_string(),
                    functions: vec![FileFunctionDef {
                        module: "MyApp.Service".to_string(),
                        name: "process".to_string(),
                        arity: 1,
                        kind: "def".to_string(),
                        line: 5,
                        start_line: 5,
                        end_line: 15,
                        pattern: String::new(),
                        guard: String::new(),
                    }],
                },
            ],
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    crate::output_table_test! {
        test_name: test_to_table_empty,
        fixture: empty_result,
        fixture_type: FileResult,
        expected: EMPTY_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_single_file,
        fixture: single_file_result,
        fixture_type: FileResult,
        expected: SINGLE_FILE_TABLE,
    }

    crate::output_table_test! {
        test_name: test_to_table_multiple_files,
        fixture: multiple_files_result,
        fixture_type: FileResult,
        expected: MULTIPLE_FILES_TABLE,
    }

    crate::output_table_test! {
        test_name: test_format_json,
        fixture: single_file_result,
        fixture_type: FileResult,
        expected: crate::test_utils::load_output_fixture("file", "single.json"),
        format: Json,
    }

    crate::output_table_test! {
        test_name: test_format_toon,
        fixture: single_file_result,
        fixture_type: FileResult,
        expected: crate::test_utils::load_output_fixture("file", "single.toon"),
        format: Toon,
    }

    crate::output_table_test! {
        test_name: test_format_toon_empty,
        fixture: empty_result,
        fixture_type: FileResult,
        expected: crate::test_utils::load_output_fixture("file", "empty.toon"),
        format: Toon,
    }
}
