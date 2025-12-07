use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::StructCmd;
use crate::commands::Execute;
use crate::db::{extract_bool, extract_string, extract_string_or, open_db, run_query, Params};

#[derive(Error, Debug)]
enum StructError {
    #[error("Struct query failed: {message}")]
    QueryFailed { message: String },
}

/// A struct field definition
#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub project: String,
    pub module: String,
    pub field: String,
    pub default_value: String,
    pub required: bool,
    pub inferred_type: String,
}

/// A struct with all its fields grouped
#[derive(Debug, Clone, Serialize)]
pub struct StructDefinition {
    pub project: String,
    pub module: String,
    pub fields: Vec<FieldInfo>,
}

/// Field information within a struct
#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
    pub name: String,
    pub default_value: String,
    pub required: bool,
    pub inferred_type: String,
}

/// Result of the struct command execution
#[derive(Debug, Default, Serialize)]
pub struct StructResult {
    pub module_pattern: String,
    pub structs: Vec<StructDefinition>,
}

impl Execute for StructCmd {
    type Output = StructResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = StructResult {
            module_pattern: self.module.clone(),
            ..Default::default()
        };

        let fields = find_struct_fields(
            &db,
            &self.module,
            &self.project,
            self.regex,
            self.limit,
        )?;

        // Group fields by (project, module)
        result.structs = group_fields_into_structs(fields);

        Ok(result)
    }
}

fn find_struct_fields(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<StructField>, Box<dyn Error>> {
    let module_cond = if use_regex {
        "regex_matches(module, $module_pattern)".to_string()
    } else {
        "module == $module_pattern".to_string()
    };

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, module, field, default_value, required, inferred_type] :=
            *struct_fields{{project, module, field, default_value, required, inferred_type}},
            {module_cond}
            {project_cond}
        :order module, field
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(&db, &script, params).map_err(|e| StructError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(field) = extract_string(&row[2]) else { continue };
            let default_value = extract_string_or(&row[3], "");
            let required = extract_bool(&row[4], false);
            let inferred_type = extract_string_or(&row[5], "");

            results.push(StructField {
                project,
                module,
                field,
                default_value,
                required,
                inferred_type,
            });
        }
    }

    Ok(results)
}

fn group_fields_into_structs(fields: Vec<StructField>) -> Vec<StructDefinition> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<(String, String), Vec<FieldInfo>> = BTreeMap::new();

    for field in fields {
        let key = (field.project.clone(), field.module.clone());
        grouped.entry(key).or_default().push(FieldInfo {
            name: field.field,
            default_value: field.default_value,
            required: field.required,
            inferred_type: field.inferred_type,
        });
    }

    grouped
        .into_iter()
        .map(|((project, module), fields)| StructDefinition {
            project,
            module,
            fields,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::ImportCmd;
    use crate::commands::Execute;
    use rstest::{fixture, rstest};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_call_graph_json() -> &'static str {
        r#"{
            "structs": {
                "MyApp.User": {
                    "fields": [
                        {"default": "nil", "field": "id", "required": true, "inferred_type": "integer()"},
                        {"default": "nil", "field": "name", "required": false, "inferred_type": "String.t()"},
                        {"default": "nil", "field": "email", "required": true, "inferred_type": "String.t()"}
                    ]
                },
                "MyApp.Post": {
                    "fields": [
                        {"default": "nil", "field": "title", "required": true, "inferred_type": "String.t()"},
                        {"default": "nil", "field": "body", "required": false, "inferred_type": "String.t()"}
                    ]
                }
            },
            "function_locations": {},
            "calls": [],
            "type_signatures": {}
        }"#
    }

    fn create_temp_json_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write temp file");
        file
    }

    #[fixture]
    fn populated_db() -> NamedTempFile {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let json_file = create_temp_json_file(sample_call_graph_json());

        let import_cmd = ImportCmd {
            file: json_file.path().to_path_buf(),
            project: "test_project".to_string(),
            clear: false,
        };
        import_cmd
            .execute(db_file.path())
            .expect("Import should succeed");

        db_file
    }

    #[rstest]
    fn test_struct_exact_match(populated_db: NamedTempFile) {
        let cmd = StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Struct should succeed");
        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.structs[0].module, "MyApp.User");
        assert_eq!(result.structs[0].fields.len(), 3); // id, name, email
    }

    #[rstest]
    fn test_struct_fields_content(populated_db: NamedTempFile) {
        let cmd = StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Struct should succeed");
        let user_struct = &result.structs[0];

        // Find the email field
        let email_field = user_struct.fields.iter().find(|f| f.name == "email").unwrap();
        assert!(email_field.required);
        assert_eq!(email_field.inferred_type, "String.t()");
    }

    #[rstest]
    fn test_struct_regex_match(populated_db: NamedTempFile) {
        let cmd = StructCmd {
            module: "MyApp\\..*".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Struct should succeed");
        assert_eq!(result.structs.len(), 2); // User and Post
    }

    #[rstest]
    fn test_struct_no_match(populated_db: NamedTempFile) {
        let cmd = StructCmd {
            module: "NonExistent".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Struct should succeed");
        assert!(result.structs.is_empty());
    }

    #[rstest]
    fn test_struct_with_project_filter(populated_db: NamedTempFile) {
        let cmd = StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(populated_db.path()).expect("Struct should succeed");
        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.structs[0].project, "test_project");
    }

    #[rstest]
    fn test_struct_with_limit(populated_db: NamedTempFile) {
        let cmd = StructCmd {
            module: "MyApp\\..*".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 3, // Limit to 3 fields total
        };
        let result = cmd.execute(populated_db.path()).expect("Struct should succeed");
        // With limit=3, we get at most 3 fields total across all structs
        let total_fields: usize = result.structs.iter().map(|s| s.fields.len()).sum();
        assert!(total_fields <= 3);
    }

    #[rstest]
    fn test_struct_empty_db() {
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let cmd = StructCmd {
            module: "MyApp".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        };
        let result = cmd.execute(db_file.path());
        assert!(result.is_err());
    }
}
