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
    use rstest::{fixture, rstest};

    const TEST_JSON: &str = r#"{
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
    }"#;

    crate::execute_test_fixture! {
        fixture_name: populated_db,
        json: TEST_JSON,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_struct_exact_match,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.structs.len(), 1);
            assert_eq!(result.structs[0].module, "MyApp.User");
            assert_eq!(result.structs[0].fields.len(), 3);
        },
    }

    crate::execute_test! {
        test_name: test_struct_fields_content,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            let user_struct = &result.structs[0];
            let email_field = user_struct.fields.iter().find(|f| f.name == "email").unwrap();
            assert!(email_field.required);
            assert_eq!(email_field.inferred_type, "String.t()");
        },
    }

    crate::execute_count_test! {
        test_name: test_struct_regex_match,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp\\..*".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 100,
        },
        field: structs,
        expected: 2,
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_struct_no_match,
        fixture: populated_db,
        cmd: StructCmd {
            module: "NonExistent".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        empty_field: structs,
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_struct_with_project_filter,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp.User".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
        assertions: |result| {
            assert_eq!(result.structs.len(), 1);
            assert_eq!(result.structs[0].project, "test_project");
        },
    }

    crate::execute_test! {
        test_name: test_struct_with_limit,
        fixture: populated_db,
        cmd: StructCmd {
            module: "MyApp\\..*".to_string(),
            project: "test_project".to_string(),
            regex: true,
            limit: 3,
        },
        assertions: |result| {
            let total_fields: usize = result.structs.iter().map(|s| s.fields.len()).sum();
            assert!(total_fields <= 3);
        },
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: StructCmd,
        cmd: StructCmd {
            module: "MyApp".to_string(),
            project: "test_project".to_string(),
            regex: false,
            limit: 100,
        },
    }
}
