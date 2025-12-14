use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum FunctionError {
    #[error("Function query failed: {message}")]
    QueryFailed { message: String },
}

/// A function signature
#[derive(Debug, Clone, Serialize)]
pub struct FunctionSignature {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub args: String,
    pub return_type: String,
}

/// Query builder for finding functions by module/function pattern and arity
#[derive(Debug)]
pub struct FunctionQueryBuilder {
    pub module_pattern: String,
    pub function_pattern: String,
    pub arity: Option<i64>,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for FunctionQueryBuilder {
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
        params.insert("function_pattern".to_string(), DataValue::Str(self.function_pattern.clone().into()));
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));

        if let Some(a) = self.arity {
            params.insert("arity".to_string(), DataValue::from(a));
        }
        params
    }
}

impl FunctionQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let module_cond = crate::utils::ConditionBuilder::new("module", "module_pattern").build(self.use_regex);
        let function_cond = crate::utils::ConditionBuilder::new("name", "function_pattern")
            .with_leading_comma()
            .build(self.use_regex);
        let arity_cond = crate::utils::OptionalConditionBuilder::new("arity", "arity")
            .with_leading_comma()
            .build(self.arity.is_some());

        Ok(format!(
            r#"?[project, module, name, arity, args, return_type] :=
    *functions{{project, module, name, arity, args, return_type}},
    {module_cond}
    {function_cond}
    {arity_cond}
    , project == $project
:order module, name, arity
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };
        let fn_match = if self.use_regex { "=~" } else { "CONTAINS" };

        let mut conditions = vec![
            format!("f.module {} $module_pattern", mod_match),
            format!("f.name {} $function_pattern", fn_match),
            "f.project = $project".to_string(),
        ];

        if self.arity.is_some() {
            conditions.push("f.arity = $arity".to_string());
        }

        Ok(format!(
            r#"MATCH (f:Function)
WHERE {}
RETURN f.project, f.module, f.name, f.arity, f.args, f.return_type
ORDER BY f.module, f.name, f.arity
LIMIT {}"#,
            conditions.join(" AND "),
            self.limit
        ))
    }
}

pub fn find_functions(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionSignature>, Box<dyn Error>> {
    let builder = FunctionQueryBuilder {
        module_pattern: module_pattern.to_string(),
        function_pattern: function_pattern.to_string(),
        arity,
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| FunctionError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let args = extract_string_or(&row[4], "");
            let return_type = extract_string_or(&row[5], "");

            results.push(FunctionSignature {
                project,
                module,
                name,
                arity,
                args,
                return_type,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_function_query_cozo_basic() {
        let builder = FunctionQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: "handle".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("?[project, module, name, arity, args, return_type]"));
        assert!(compiled.contains("*functions"));
        assert!(compiled.contains("module == $module_pattern"));
        assert!(compiled.contains(":order module, name, arity"));
    }

    #[test]
    fn test_function_query_cozo_with_arity() {
        let builder = FunctionQueryBuilder {
            module_pattern: "MyApp".to_string(),
            function_pattern: "init".to_string(),
            arity: Some(1),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 10,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("arity == $arity"));
    }

    #[test]
    fn test_function_query_cozo_regex() {
        let builder = FunctionQueryBuilder {
            module_pattern: "MyApp\\..*".to_string(),
            function_pattern: "handle_.*".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: true,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(module, $module_pattern)"));
        assert!(compiled.contains("regex_matches(name, $function_pattern)"));
    }

    #[test]
    fn test_function_query_age_basic() {
        let builder = FunctionQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: "handle".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH (f:Function)"));
        assert!(compiled.contains("f.module = $module_pattern"));
        assert!(compiled.contains("f.name CONTAINS $function_pattern"));
        assert!(compiled.contains("ORDER BY f.module, f.name, f.arity"));
    }

    #[test]
    fn test_function_query_age_regex() {
        let builder = FunctionQueryBuilder {
            module_pattern: "MyApp\\..*".to_string(),
            function_pattern: "handle_.*".to_string(),
            arity: Some(2),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 20,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("f.module =~ $module_pattern"));
        assert!(compiled.contains("f.name =~ $function_pattern"));
        assert!(compiled.contains("f.arity = $arity"));
    }

    #[test]
    fn test_function_query_parameters() {
        let builder = FunctionQueryBuilder {
            module_pattern: "mod".to_string(),
            function_pattern: "func".to_string(),
            arity: Some(3),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 4);
        assert!(params.contains_key("module_pattern"));
        assert!(params.contains_key("function_pattern"));
        assert!(params.contains_key("project"));
        assert!(params.contains_key("arity"));
    }

    #[test]
    fn test_function_query_parameters_without_arity() {
        let builder = FunctionQueryBuilder {
            module_pattern: "mod".to_string(),
            function_pattern: "func".to_string(),
            arity: None,
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 3);
        assert!(!params.contains_key("arity"));
    }
}
