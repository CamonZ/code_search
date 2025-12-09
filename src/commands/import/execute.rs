use std::error::Error;
use std::fs;

use cozo::DbInstance;

use super::ImportCmd;
use crate::commands::Execute;
use crate::queries::import::{clear_project_data, import_graph, ImportError, ImportResult};
use crate::queries::import_models::CallGraph;

impl Execute for ImportCmd {
    type Output = ImportResult;

    fn execute(self, db: &DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        // Read and parse call graph
        let content = fs::read_to_string(&self.file).map_err(|e| ImportError::FileReadFailed {
            path: self.file.display().to_string(),
            message: e.to_string(),
        })?;

        let graph: CallGraph =
            serde_json::from_str(&content).map_err(|e| ImportError::JsonParseFailed {
                message: e.to_string(),
            })?;

        // Clear existing data if requested
        if self.clear {
            clear_project_data(db, &self.project)?;
        }

        // Import data
        let mut result = import_graph(db, &self.project, &graph)?;
        result.cleared = self.clear;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use rstest::{fixture, rstest};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_call_graph_json() -> &'static str {
        r#"{
            "structs": {
                "MyApp.User": {
                    "fields": [
                        {"default": "nil", "field": "name", "required": true, "inferred_type": "binary()"},
                        {"default": "0", "field": "age", "required": false, "inferred_type": "integer()"}
                    ]
                }
            },
            "function_locations": {
                "MyApp.Accounts": {
                    "get_user/1:10": {
                        "file": "lib/my_app/accounts.ex",
                        "column": 7,
                        "kind": "def",
                        "line": 10,
                        "start_line": 10,
                        "end_line": 15,
                        "pattern": "id",
                        "guard": null,
                        "source_sha": "",
                        "ast_sha": ""
                    }
                }
            },
            "calls": [
                {
                    "caller": {
                        "function": "get_user/1",
                        "line": 12,
                        "module": "MyApp.Accounts",
                        "file": "lib/my_app/accounts.ex",
                        "column": 5
                    },
                    "type": "remote",
                    "callee": {
                        "arity": 2,
                        "function": "get",
                        "module": "MyApp.Repo"
                    }
                }
            ],
            "specs": {
                "MyApp.Accounts": [
                    {
                        "arity": 1,
                        "name": "get_user",
                        "line": 9,
                        "kind": "spec",
                        "clauses": [
                            {"full": "@spec get_user(integer()) :: dynamic()", "inputs_string": ["integer()"], "return_string": "dynamic()"}
                        ]
                    }
                ]
            }
        }"#
    }

    fn create_temp_json_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write temp file");
        file
    }

    #[fixture]
    fn json_file() -> NamedTempFile {
        create_temp_json_file(sample_call_graph_json())
    }

    #[fixture]
    fn db_file() -> NamedTempFile {
        NamedTempFile::new().expect("Failed to create temp db file")
    }

    #[fixture]
    fn import_result(json_file: NamedTempFile, db_file: NamedTempFile) -> ImportResult {
        let cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };
        let db = open_db(db_file.path()).expect("Failed to open db");
        cmd.execute(&db).expect("Import should succeed")
    }

    #[rstest]
    fn test_import_creates_schemas(import_result: ImportResult) {
        assert!(!import_result.schemas.created.is_empty() || !import_result.schemas.already_existed.is_empty());
    }

    #[rstest]
    fn test_import_modules(import_result: ImportResult) {
        assert_eq!(import_result.modules_imported, 2); // MyApp.Accounts + MyApp.User (from structs)
    }

    #[rstest]
    fn test_import_functions(import_result: ImportResult) {
        assert_eq!(import_result.functions_imported, 1); // get_user/1
    }

    #[rstest]
    fn test_import_calls(import_result: ImportResult) {
        assert_eq!(import_result.calls_imported, 1);
    }

    #[rstest]
    fn test_import_structs(import_result: ImportResult) {
        assert_eq!(import_result.structs_imported, 2); // 2 fields in MyApp.User
    }

    #[rstest]
    fn test_import_function_locations(import_result: ImportResult) {
        assert_eq!(import_result.function_locations_imported, 1);
    }

    #[rstest]
    fn test_import_with_clear_flag(json_file: NamedTempFile, db_file: NamedTempFile) {
        // First import
        let cmd1 = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };
        let db = open_db(db_file.path()).expect("Failed to open db");
        cmd1.execute(&db)
            .expect("First import should succeed");

        // Second import with clear
        let cmd2 = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: true,
        };
        let result = cmd2
            .execute(&db)
            .expect("Second import should succeed");

        assert!(result.cleared);
        assert_eq!(result.modules_imported, 2);
    }

    #[rstest]
    fn test_import_empty_graph(db_file: NamedTempFile) {
        let empty_json = r#"{
            "structs": {},
            "function_locations": {},
            "calls": [],
            "type_signatures": {}
        }"#;

        let json_file = create_temp_json_file(empty_json);

        let cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };

        let db = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(&db).expect("Import should succeed");

        assert_eq!(result.modules_imported, 0);
        assert_eq!(result.functions_imported, 0);
        assert_eq!(result.calls_imported, 0);
        assert_eq!(result.structs_imported, 0);
        assert_eq!(result.function_locations_imported, 0);
    }

    #[rstest]
    fn test_import_invalid_json_fails(db_file: NamedTempFile) {
        let invalid_json = "{ not valid json }";
        let json_file = create_temp_json_file(invalid_json);

        let cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };

        let db = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(&db);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_import_nonexistent_file_fails(db_file: NamedTempFile) {
        let cmd = ImportCmd {
            file: "/nonexistent/path/call_graph.json".into(),
            project: "test_project".to_string(),
            clear: false,
        };

        let db = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(&db);
        assert!(result.is_err());
    }
}
