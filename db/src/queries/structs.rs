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
