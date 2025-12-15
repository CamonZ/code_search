use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum ReturnsError {
    #[error("Returns query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with its return type specification
#[derive(Debug, Clone, Serialize)]
pub struct ReturnEntry {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub return_string: String,
    pub line: i64,
}

/// Query builder for finding functions that return a specific type
#[derive(Debug)]
pub struct ReturnsQueryBuilder {
    pub pattern: String,
    pub project: String,
    pub use_regex: bool,
    pub module_pattern: Option<String>,
    pub limit: u32,
}

impl QueryBuilder for ReturnsQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("pattern".to_string(), DataValue::Str(self.pattern.clone().into()));
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        if let Some(ref mod_pat) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(mod_pat.clone().into()));
        }
        params
    }
}

impl ReturnsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build pattern matching function for return type
        let match_fn = if self.use_regex {
            "regex_matches(return_string, $pattern)"
        } else {
            "str_includes(return_string, $pattern)"
        };

        // Build module filter
        let module_filter = match &self.module_pattern {
            Some(_) if self.use_regex => "regex_matches(module, $module_pattern)",
            Some(_) => "str_includes(module, $module_pattern)",
            None => "true",
        };

        Ok(format!(
            r#"?[project, module, name, arity, return_string, line] :=
    *specs{{project, module, name, arity, return_string, line}},
    project == $project,
    {match_fn},
    {module_filter}
:order module, name, arity
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices only, not edges.
        // Spec vertex has: module, name, arity, return_string, line

        let pattern_op = if self.use_regex { "=~" } else { "CONTAINS" };
        let module_op = if self.use_regex { "=~" } else { "CONTAINS" };

        let mut conditions = vec![
            "s.project = $project".to_string(),
            format!("s.return_string {} $pattern", pattern_op),
        ];

        if self.module_pattern.is_some() {
            conditions.push(format!("s.module {} $module_pattern", module_op));
        }

        Ok(format!(
            r#"MATCH (s:Spec)
WHERE {}
RETURN s.project, s.module, s.name, s.arity, s.return_string, s.line
ORDER BY s.module, s.name, s.arity
LIMIT {}"#,
            conditions.join(" AND "),
            self.limit
        ))
    }
}

pub fn find_returns(
    db: &dyn DatabaseBackend,
    pattern: &str,
    project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<ReturnEntry>, Box<dyn Error>> {
    let builder = ReturnsQueryBuilder {
        pattern: pattern.to_string(),
        project: project.to_string(),
        use_regex,
        module_pattern: module_pattern.map(|s| s.to_string()),
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| ReturnsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
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
            let return_string = extract_string(&row[4]).unwrap_or_default();
            let line = extract_i64(&row[5], 0);

            results.push(ReturnEntry {
                project,
                module,
                name,
                arity,
                return_string,
                line,
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
    fn test_returns_query_cozo_basic() {
        let builder = ReturnsQueryBuilder {
            pattern: "User".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            module_pattern: None,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("*specs"));
        assert!(compiled.contains("str_includes(return_string, $pattern)"));
    }

    #[test]
    fn test_returns_query_cozo_regex() {
        let builder = ReturnsQueryBuilder {
            pattern: "User|Account".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            module_pattern: None,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(return_string, $pattern)"));
    }

    #[test]
    fn test_returns_query_cozo_with_module_pattern() {
        let builder = ReturnsQueryBuilder {
            pattern: "DateTime".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            module_pattern: Some("MyApp".to_string()),
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("str_includes(module, $module_pattern)"));
    }

    #[test]
    fn test_returns_query_age() {
        let builder = ReturnsQueryBuilder {
            pattern: "User".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            module_pattern: None,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (s:Spec)"));
        assert!(compiled.contains("s.return_string CONTAINS $pattern"));
    }

    #[test]
    fn test_returns_query_parameters() {
        let builder = ReturnsQueryBuilder {
            pattern: "MyType".to_string(),
            project: "proj".to_string(),
            use_regex: false,
            module_pattern: Some("test".to_string()),
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 3);
        assert!(params.contains_key("pattern"));
        assert!(params.contains_key("project"));
        assert!(params.contains_key("module_pattern"));
    }
}
