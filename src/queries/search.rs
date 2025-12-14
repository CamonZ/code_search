use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Search failed: {message}")]
    QueryFailed { message: String },
}

/// A module search result
#[derive(Debug, Clone, Serialize)]
pub struct ModuleResult {
    pub project: String,
    pub name: String,
    pub source: String,
}

/// A function search result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionResult {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub return_type: String,
}

/// Query builder for searching modules by name pattern
#[derive(Debug)]
pub struct SearchModulesQueryBuilder {
    pub pattern: String,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for SearchModulesQueryBuilder {
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
        params
    }
}

impl SearchModulesQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let match_fn = if self.use_regex { "regex_matches" } else { "str_includes" };
        Ok(format!(
            r#"?[project, name, source] := *modules{{project, name, source}},
    project = $project,
    {match_fn}(name, $pattern)
:limit {}
:order name"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let match_op = if self.use_regex { "=~" } else { "CONTAINS" };
        Ok(format!(
            r#"MATCH (m:Module)
WHERE m.project = $project AND m.name {} $pattern
RETURN m.project, m.name, m.source
ORDER BY m.name
LIMIT {}"#,
            match_op, self.limit
        ))
    }
}

/// Query builder for searching functions by name pattern
#[derive(Debug)]
pub struct SearchFunctionsQueryBuilder {
    pub pattern: String,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for SearchFunctionsQueryBuilder {
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
        params
    }
}

impl SearchFunctionsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let match_fn = if self.use_regex { "regex_matches" } else { "str_includes" };
        Ok(format!(
            r#"?[project, module, name, arity, return_type] := *functions{{project, module, name, arity, return_type}},
    project = $project,
    {match_fn}(name, $pattern)
:limit {}
:order module, name, arity"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let match_op = if self.use_regex { "=~" } else { "CONTAINS" };
        Ok(format!(
            r#"MATCH (f:Function)
WHERE f.project = $project AND f.name {} $pattern
RETURN f.project, f.module, f.name, f.arity, f.return_type
ORDER BY f.module, f.name, f.arity
LIMIT {}"#,
            match_op, self.limit
        ))
    }
}

pub fn search_modules(
    db: &dyn DatabaseBackend,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<ModuleResult>, Box<dyn Error>> {
    let builder = SearchModulesQueryBuilder {
        pattern: pattern.to_string(),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 3 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let source = extract_string_or(&row[2], "unknown");
            results.push(ModuleResult { project, name, source });
        }
    }

    Ok(results)
}

pub fn search_functions(
    db: &dyn DatabaseBackend,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<FunctionResult>, Box<dyn Error>> {
    let builder = SearchFunctionsQueryBuilder {
        pattern: pattern.to_string(),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 5 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let return_type = extract_string_or(&row[4], "");
            results.push(FunctionResult {
                project,
                module,
                name,
                arity,
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
    fn test_search_modules_cozo_compilation() {
        let builder = SearchModulesQueryBuilder {
            pattern: "test".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 10,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("?[project, name, source]"));
        assert!(compiled.contains("*modules"));
        assert!(compiled.contains("str_includes"));
        assert!(compiled.contains(":limit 10"));
    }

    #[test]
    fn test_search_modules_cozo_compilation_regex() {
        let builder = SearchModulesQueryBuilder {
            pattern: "test.*".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 5,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_search_modules_age_compilation() {
        let builder = SearchModulesQueryBuilder {
            pattern: "test".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 10,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH (m:Module)"));
        assert!(compiled.contains("CONTAINS"));
        assert!(compiled.contains("RETURN m.project, m.name, m.source"));
        assert!(compiled.contains("LIMIT 10"));
    }

    #[test]
    fn test_search_modules_age_compilation_regex() {
        let builder = SearchModulesQueryBuilder {
            pattern: "test.*".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 10,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("=~"));
    }

    #[test]
    fn test_search_functions_cozo_compilation() {
        let builder = SearchFunctionsQueryBuilder {
            pattern: "foo".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 20,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("?[project, module, name, arity, return_type]"));
        assert!(compiled.contains("*functions"));
        assert!(compiled.contains(":order module, name, arity"));
    }

    #[test]
    fn test_search_functions_cozo_compilation_regex() {
        let builder = SearchFunctionsQueryBuilder {
            pattern: "foo.*".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 20,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_search_functions_age_compilation() {
        let builder = SearchFunctionsQueryBuilder {
            pattern: "foo".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 20,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH (f:Function)"));
        assert!(compiled.contains("CONTAINS"));
        assert!(compiled.contains("RETURN f.project, f.module, f.name, f.arity, f.return_type"));
        assert!(compiled.contains("ORDER BY f.module, f.name, f.arity"));
    }

    #[test]
    fn test_search_functions_age_compilation_regex() {
        let builder = SearchFunctionsQueryBuilder {
            pattern: "foo".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 20,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("=~")); // regex operator
        assert!(compiled.contains("ORDER BY f.module, f.name, f.arity"));
    }

    #[test]
    fn test_search_modules_parameters() {
        let builder = SearchModulesQueryBuilder {
            pattern: "test_pattern".to_string(),
            project: "test_project".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("pattern"));
        assert!(params.contains_key("project"));
    }

    #[test]
    fn test_search_functions_parameters() {
        let builder = SearchFunctionsQueryBuilder {
            pattern: "test_pattern".to_string(),
            project: "test_project".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("pattern"));
        assert!(params.contains_key("project"));
    }
}
