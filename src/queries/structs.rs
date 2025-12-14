use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_bool, extract_string, extract_string_or, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

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

/// Query builder for finding struct fields by module pattern
#[derive(Debug)]
pub struct StructsQueryBuilder {
    pub module_pattern: String,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for StructsQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("module_pattern".to_string(), DataValue::Str(self.module_pattern.clone().into()));
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        params
    }
}

impl StructsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let module_cond = if self.use_regex {
            "regex_matches(module, $module_pattern)".to_string()
        } else {
            "module == $module_pattern".to_string()
        };

        Ok(format!(
            r#"?[project, module, field, default_value, required, inferred_type] :=
    *struct_fields{{project, module, field, default_value, required, inferred_type}},
    {module_cond},
    project == $project
:order module, field
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };

        Ok(format!(
            r#"MATCH (f:StructField)
WHERE f.project = $project
  AND f.module {} $module_pattern
RETURN f.project, f.module, f.field, f.default_value, f.required, f.inferred_type
ORDER BY f.module, f.field
LIMIT {}"#,
            mod_match, self.limit
        ))
    }
}

pub fn find_struct_fields(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<StructField>, Box<dyn Error>> {
    let builder = StructsQueryBuilder {
        module_pattern: module_pattern.to_string(),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| StructError::QueryFailed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_structs_query_cozo_exact_match() {
        let builder = StructsQueryBuilder {
            module_pattern: "MyApp.User".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("*struct_fields"));
        assert!(compiled.contains("module == $module_pattern"));
    }

    #[test]
    fn test_structs_query_cozo_regex() {
        let builder = StructsQueryBuilder {
            module_pattern: "MyApp.*".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(module, $module_pattern)"));
    }

    #[test]
    fn test_structs_query_age() {
        let builder = StructsQueryBuilder {
            module_pattern: "MyApp.User".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH"));
        assert!(compiled.contains("StructField"));
    }

    #[test]
    fn test_structs_query_parameters() {
        let builder = StructsQueryBuilder {
            module_pattern: "test".to_string(),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("module_pattern"));
        assert!(params.contains_key("project"));
    }
}
