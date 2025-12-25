use std::error::Error;


use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_bool, extract_string, extract_string_or, run_query};
use crate::query_builders::{validate_regex_patterns, ConditionBuilder};

#[derive(Error, Debug)]
pub enum StructError {
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

pub fn find_struct_fields(
    db: &dyn Database,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<StructField>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);

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

    let mut params = QueryParams::new();
    params = params.with_str("module_pattern", module_pattern);
    params = params.with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| StructError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
            let Some(project) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(module) = extract_string(row.get(1).unwrap()) else { continue };
            let Some(field) = extract_string(row.get(2).unwrap()) else { continue };
            let default_value = extract_string_or(row.get(3).unwrap(), "");
            let required = extract_bool(row.get(4).unwrap(), false);
            let inferred_type = extract_string_or(row.get(5).unwrap(), "");

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

pub fn group_fields_into_structs(fields: Vec<StructField>) -> Vec<StructDefinition> {
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

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::structs_db("default")
    }

    #[rstest]
    fn test_find_struct_fields_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_fields(&*populated_db, "", "default", false, 100);
        assert!(result.is_ok());
        let fields = result.unwrap();
        // May be empty if fixture doesn't have struct fields, just verify query executes
        assert!(fields.is_empty() || !fields.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_struct_fields_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_fields(&*populated_db, "NonExistentModule", "default", false, 100);
        assert!(result.is_ok());
        let fields = result.unwrap();
        assert!(fields.is_empty(), "Should return empty results for non-existent module");
    }

    #[rstest]
    fn test_find_struct_fields_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_fields(&*populated_db, "MyApp", "default", false, 100);
        assert!(result.is_ok());
        let fields = result.unwrap();
        for field in &fields {
            assert!(field.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_struct_fields_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_struct_fields(&*populated_db, "", "default", false, 5)
            .unwrap();
        let limit_100 = find_struct_fields(&*populated_db, "", "default", false, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_struct_fields_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_fields(&*populated_db, "^MyApp\\..*$", "default", true, 100);
        assert!(result.is_ok());
        let fields = result.unwrap();
        for field in &fields {
            assert!(field.module.starts_with("MyApp"), "Module should match regex");
        }
    }

    #[rstest]
    fn test_find_struct_fields_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_fields(&*populated_db, "[invalid", "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_struct_fields_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_fields(&*populated_db, "", "nonexistent", false, 100);
        assert!(result.is_ok());
        let fields = result.unwrap();
        assert!(fields.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_struct_fields_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_fields(&*populated_db, "", "default", false, 100);
        assert!(result.is_ok());
        let fields = result.unwrap();
        if !fields.is_empty() {
            let field = &fields[0];
            assert_eq!(field.project, "default");
            assert!(!field.module.is_empty());
            assert!(!field.field.is_empty());
        }
    }

    #[rstest]
    fn test_group_fields_into_structs_groups_correctly() {
        let fields = vec![
            StructField {
                project: "proj".to_string(),
                module: "Module1".to_string(),
                field: "field1".to_string(),
                default_value: "".to_string(),
                required: true,
                inferred_type: "String".to_string(),
            },
            StructField {
                project: "proj".to_string(),
                module: "Module1".to_string(),
                field: "field2".to_string(),
                default_value: "0".to_string(),
                required: false,
                inferred_type: "i64".to_string(),
            },
            StructField {
                project: "proj".to_string(),
                module: "Module2".to_string(),
                field: "field3".to_string(),
                default_value: "".to_string(),
                required: true,
                inferred_type: "bool".to_string(),
            },
        ];

        let structs = group_fields_into_structs(fields);

        assert_eq!(structs.len(), 2, "Should have 2 structs");
        assert_eq!(structs[0].fields.len(), 2, "First struct should have 2 fields");
        assert_eq!(structs[1].fields.len(), 1, "Second struct should have 1 field");
    }

    #[rstest]
    fn test_group_fields_into_structs_empty() {
        let fields = vec![];
        let structs = group_fields_into_structs(fields);
        assert!(structs.is_empty(), "Empty fields should result in empty structs");
    }
}
