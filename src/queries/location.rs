use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::{DataValue, Num};
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum LocationError {
    #[error("Location query failed: {message}")]
    QueryFailed { message: String },
}

/// A function location result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionLocation {
    pub project: String,
    pub file: String,
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub module: String,
    pub kind: String,
    pub name: String,
    pub arity: i64,
    pub pattern: String,
    pub guard: String,
}

/// Query builder for finding function locations by pattern with optional module and arity filters
#[derive(Debug)]
pub struct LocationQueryBuilder {
    pub module_pattern: Option<String>,
    pub function_pattern: String,
    pub arity: Option<i64>,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for LocationQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("function_pattern".to_string(), DataValue::Str(self.function_pattern.clone().into()));
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));

        if let Some(ref mod_pat) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(mod_pat.clone().into()));
        }
        if let Some(a) = self.arity {
            params.insert("arity".to_string(), DataValue::Num(Num::Int(a)));
        }
        params
    }
}

impl LocationQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build conditions
        let fn_cond = if self.use_regex {
            "regex_matches(name, $function_pattern)".to_string()
        } else {
            "name == $function_pattern".to_string()
        };

        let module_cond = match &self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", module == $module_pattern".to_string(),
            None => String::new(),
        };

        let arity_cond = if self.arity.is_some() {
            ", arity == $arity"
        } else {
            ""
        };

        Ok(format!(
            r#"?[project, file, line, start_line, end_line, module, kind, name, arity, pattern, guard] :=
    *function_locations{{project, module, name, arity, line, file, kind, start_line, end_line, pattern, guard}},
    {fn_cond}
    {module_cond}
    {arity_cond}
    , project == $project
:order module, name, arity, line
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // Build WHERE conditions
        let fn_match = if self.use_regex { "=~" } else { "=" };
        let mut conditions = vec![
            format!("fl.name {} $function_pattern", fn_match),
            "fl.project = $project".to_string(),
        ];

        if self.module_pattern.is_some() {
            let mod_match = if self.use_regex { "=~" } else { "=" };
            conditions.push(format!("fl.module {} $module_pattern", mod_match));
        }

        if self.arity.is_some() {
            conditions.push("fl.arity = $arity".to_string());
        }

        Ok(format!(
            r#"MATCH (fl:FunctionLocation)
WHERE {}
RETURN fl.project, fl.file, fl.line, fl.start_line, fl.end_line,
       fl.module, fl.kind, fl.name, fl.arity, fl.pattern, fl.guard
ORDER BY fl.module, fl.name, fl.arity, fl.line
LIMIT {}"#,
            conditions.join(" AND "),
            self.limit
        ))
    }
}

pub fn find_locations(
    db: &dyn DatabaseBackend,
    module_pattern: Option<&str>,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionLocation>, Box<dyn Error>> {
    let builder = LocationQueryBuilder {
        module_pattern: module_pattern.map(String::from),
        function_pattern: function_pattern.to_string(),
        arity,
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| LocationError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 11 {
            // Order matches query: project, file, line, start_line, end_line, module, kind, name, arity, pattern, guard
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(file) = extract_string(&row[1]) else { continue };
            let line = extract_i64(&row[2], 0);
            let start_line = extract_i64(&row[3], 0);
            let end_line = extract_i64(&row[4], 0);
            let Some(module) = extract_string(&row[5]) else { continue };
            let kind = extract_string_or(&row[6], "");
            let Some(name) = extract_string(&row[7]) else { continue };
            let arity = extract_i64(&row[8], 0);
            let pattern = extract_string_or(&row[9], "");
            let guard = extract_string_or(&row[10], "");

            results.push(FunctionLocation {
                project,
                file,
                line,
                start_line,
                end_line,
                module,
                kind,
                name,
                arity,
                pattern,
                guard,
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
    fn test_location_cozo_basic() {
        let builder = LocationQueryBuilder {
            module_pattern: None,
            function_pattern: "handle_call".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("?[project, file, line, start_line, end_line, module, kind, name, arity, pattern, guard]"));
        assert!(compiled.contains("*function_locations"));
        assert!(compiled.contains("name == $function_pattern"));
        assert!(compiled.contains(":limit 100"));
    }

    #[test]
    fn test_location_cozo_with_module() {
        let builder = LocationQueryBuilder {
            module_pattern: Some("MyApp.Server".to_string()),
            function_pattern: "init".to_string(),
            arity: Some(1),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 10,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("module == $module_pattern"));
        assert!(compiled.contains("arity == $arity"));
    }

    #[test]
    fn test_location_cozo_regex() {
        let builder = LocationQueryBuilder {
            module_pattern: Some("MyApp\\..*".to_string()),
            function_pattern: "handle_.*".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: true,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(name, $function_pattern)"));
        assert!(compiled.contains("regex_matches(module, $module_pattern)"));
    }

    #[test]
    fn test_location_age_basic() {
        let builder = LocationQueryBuilder {
            module_pattern: None,
            function_pattern: "handle_call".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH (fl:FunctionLocation)"));
        assert!(compiled.contains("fl.name = $function_pattern"));
        assert!(compiled.contains("fl.project = $project"));
        assert!(compiled.contains("ORDER BY fl.module, fl.name, fl.arity, fl.line"));
        assert!(compiled.contains("LIMIT 100"));
    }

    #[test]
    fn test_location_age_with_all_filters() {
        let builder = LocationQueryBuilder {
            module_pattern: Some("MyApp.Server".to_string()),
            function_pattern: "init".to_string(),
            arity: Some(1),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 10,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("fl.name =~ $function_pattern"));
        assert!(compiled.contains("fl.module =~ $module_pattern"));
        assert!(compiled.contains("fl.arity = $arity"));
    }

    #[test]
    fn test_location_parameters_minimal() {
        let builder = LocationQueryBuilder {
            module_pattern: None,
            function_pattern: "test".to_string(),
            arity: None,
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2); // function_pattern, project
        assert!(params.contains_key("function_pattern"));
        assert!(params.contains_key("project"));
        assert!(!params.contains_key("module_pattern"));
        assert!(!params.contains_key("arity"));
    }

    #[test]
    fn test_location_parameters_full() {
        let builder = LocationQueryBuilder {
            module_pattern: Some("Module".to_string()),
            function_pattern: "func".to_string(),
            arity: Some(2),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 4);
        assert!(params.contains_key("module_pattern"));
        assert!(params.contains_key("arity"));
    }
}
