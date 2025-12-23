use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_bool, extract_string, extract_string_or, run_query, Params};
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
    db: &cozo::DbInstance,
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

    let mut params = Params::new();
    params.insert("module_pattern", DataValue::Str(module_pattern.into()));
    params.insert("project", DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| StructError::QueryFailed {
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
