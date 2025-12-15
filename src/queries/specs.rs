use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum SpecsError {
    #[error("Specs query failed: {message}")]
    QueryFailed { message: String },
}

/// A spec or callback definition
#[derive(Debug, Clone, Serialize)]
pub struct SpecDef {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    pub inputs_string: String,
    pub return_string: String,
    pub full: String,
}

/// Query builder for finding specs by module pattern with optional function and kind filters
#[derive(Debug)]
pub struct SpecsQueryBuilder {
    pub module_pattern: String,
    pub function_pattern: Option<String>,
    pub kind_filter: Option<String>,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for SpecsQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        params.insert("module_pattern".to_string(), DataValue::Str(self.module_pattern.clone().into()));

        if let Some(ref func) = self.function_pattern {
            params.insert("function_pattern".to_string(), DataValue::Str(func.clone().into()));
        }

        if let Some(ref kind) = self.kind_filter {
            params.insert("kind".to_string(), DataValue::Str(kind.clone().into()));
        }
        params
    }
}

impl SpecsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build module filter
        let module_filter = if self.use_regex {
            "regex_matches(module, $module_pattern)"
        } else {
            "module == $module_pattern"
        };

        // Build function filter
        let function_filter = match &self.function_pattern {
            Some(_) if self.use_regex => ", regex_matches(name, $function_pattern)",
            Some(_) => ", str_includes(name, $function_pattern)",
            None => "",
        };

        // Build kind filter
        let kind_filter_sql = match &self.kind_filter {
            Some(_) => ", kind == $kind",
            None => "",
        };

        Ok(format!(
            r#"?[project, module, name, arity, kind, line, inputs_string, return_string, full] :=
    *specs{{project, module, name, arity, kind, line, inputs_string, return_string, full}},
    project == $project,
    {module_filter}
    {function_filter}
    {kind_filter_sql}
:order module, name, arity
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };

        let mut conditions = vec![
            "s.project = $project".to_string(),
            format!("s.module {} $module_pattern", mod_match),
        ];

        if self.function_pattern.is_some() {
            let fn_match = if self.use_regex { "=~" } else { "CONTAINS" };
            conditions.push(format!("s.name {} $function_pattern", fn_match));
        }

        if self.kind_filter.is_some() {
            conditions.push("s.kind = $kind".to_string());
        }

        // Note: Using "full_spec" instead of "full" because "full" is a reserved word in PostgreSQL
        Ok(format!(
            r#"MATCH (s:Spec)
WHERE {}
RETURN s.project AS project, s.module AS module, s.name AS name, s.arity AS arity,
       s.kind AS kind, s.line AS line, s.inputs_string AS inputs_string,
       s.return_string AS return_string, s.full AS full_spec
ORDER BY s.module, s.name, s.arity
LIMIT {}"#,
            conditions.join(" AND "),
            self.limit
        ))
    }
}

pub fn find_specs(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    function_pattern: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<SpecDef>, Box<dyn Error>> {
    let builder = SpecsQueryBuilder {
        module_pattern: module_pattern.to_string(),
        function_pattern: function_pattern.map(String::from),
        kind_filter: kind_filter.map(String::from),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| SpecsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 9 {
            let Some(project) = extract_string(&row[0]) else {
                continue;
            };
            let Some(module) = extract_string(&row[1]) else {
                continue;
            };
            let Some(name) = extract_string(&row[2]) else {
                continue;
            };
            let arity = extract_i64(&row[3], 0);
            let Some(kind) = extract_string(&row[4]) else {
                continue;
            };
            let line = extract_i64(&row[5], 0);
            let inputs_string = extract_string(&row[6]).unwrap_or_default();
            let return_string = extract_string(&row[7]).unwrap_or_default();
            let full = extract_string(&row[8]).unwrap_or_default();

            results.push(SpecDef {
                project,
                module,
                name,
                arity,
                kind,
                line,
                inputs_string,
                return_string,
                full,
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
    fn test_specs_query_cozo_basic() {
        let builder = SpecsQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: None,
            kind_filter: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("?[project, module, name, arity, kind, line, inputs_string, return_string, full]"));
        assert!(compiled.contains("*specs"));
        assert!(compiled.contains("module == $module_pattern"));
        assert!(!compiled.contains("$function_pattern"));
        assert!(!compiled.contains("$kind"));
    }

    #[test]
    fn test_specs_query_cozo_with_function_filter() {
        let builder = SpecsQueryBuilder {
            module_pattern: "MyApp".to_string(),
            function_pattern: Some("handle".to_string()),
            kind_filter: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("str_includes(name, $function_pattern)"));
    }

    #[test]
    fn test_specs_query_cozo_with_kind_filter() {
        let builder = SpecsQueryBuilder {
            module_pattern: "MyApp".to_string(),
            function_pattern: None,
            kind_filter: Some("callback".to_string()),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("kind == $kind"));
    }

    #[test]
    fn test_specs_query_cozo_regex() {
        let builder = SpecsQueryBuilder {
            module_pattern: "MyApp\\..*".to_string(),
            function_pattern: Some("handle_.*".to_string()),
            kind_filter: Some("spec".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(module, $module_pattern)"));
        assert!(compiled.contains("regex_matches(name, $function_pattern)"));
    }

    #[test]
    fn test_specs_query_age_basic() {
        let builder = SpecsQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: None,
            kind_filter: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH (s:Spec)"));
        assert!(compiled.contains("s.module = $module_pattern"));
        assert!(compiled.contains("s.project = $project"));
        // AGE query uses explicit aliases to avoid reserved word conflicts
        assert!(compiled.contains("RETURN s.project AS project"));
    }

    #[test]
    fn test_specs_query_age_all_filters() {
        let builder = SpecsQueryBuilder {
            module_pattern: "MyApp".to_string(),
            function_pattern: Some("handle".to_string()),
            kind_filter: Some("callback".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 25,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("s.module =~ $module_pattern"));
        assert!(compiled.contains("s.name =~ $function_pattern"));
        assert!(compiled.contains("s.kind = $kind"));
        assert!(compiled.contains("LIMIT 25"));
    }

    #[test]
    fn test_specs_query_parameters_minimal() {
        let builder = SpecsQueryBuilder {
            module_pattern: "mod".to_string(),
            function_pattern: None,
            kind_filter: None,
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2); // project, module_pattern
    }

    #[test]
    fn test_specs_query_parameters_full() {
        let builder = SpecsQueryBuilder {
            module_pattern: "mod".to_string(),
            function_pattern: Some("func".to_string()),
            kind_filter: Some("spec".to_string()),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 4);
        assert!(params.contains_key("function_pattern"));
        assert!(params.contains_key("kind"));
    }
}
