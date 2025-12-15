use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum ManyClausesError {
    #[error("Many clauses query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with clause count information
#[derive(Debug, Clone, Serialize)]
pub struct ManyClauses {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub clauses: i64,
    pub first_line: i64,
    pub last_line: i64,
    pub file: String,
    pub generated_by: String,
}

/// Query builder for finding functions with many clause definitions
#[derive(Debug)]
pub struct ManyClausesQueryBuilder {
    pub min_clauses: i64,
    pub module_pattern: Option<String>,
    pub project: String,
    pub use_regex: bool,
    pub include_generated: bool,
    pub limit: u32,
}

impl QueryBuilder for ManyClausesQueryBuilder {
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
        params.insert("min_clauses".to_string(), DataValue::from(self.min_clauses));
        if let Some(ref pattern) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(pattern.clone().into()));
        }
        params
    }
}

impl ManyClausesQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let module_filter = match self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", str_includes(module, $module_pattern)".to_string(),
            None => String::new(),
        };

        let generated_filter = if self.include_generated {
            String::new()
        } else {
            ", generated_by == \"\"".to_string()
        };

        Ok(format!(
            r#"clause_counts[module, name, arity, count(line), min(start_line), max(end_line), file, generated_by] :=
    *function_locations{{project, module, name, arity, line, start_line, end_line, file, generated_by}},
    project == $project
    {module_filter}
    {generated_filter}

?[module, name, arity, clauses, first_line, last_line, file, generated_by] :=
    clause_counts[module, name, arity, clauses, first_line, last_line, file, generated_by],
    clauses >= $min_clauses

:order -clauses, module, name
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices only, not edges.
        // FunctionLocation has all the fields we need.

        let mod_match = if self.use_regex { "=~" } else { "=" };

        let mut where_conditions = vec!["loc.project = $project".to_string()];

        if let Some(_) = &self.module_pattern {
            where_conditions.push(format!("loc.module {} $module_pattern", mod_match));
        }

        if !self.include_generated {
            where_conditions.push("loc.generated_by = ''".to_string());
        }

        let where_clause = where_conditions.join("\n  AND ");

        Ok(format!(
            r#"MATCH (loc:FunctionLocation)
WHERE {where_clause}
WITH loc.module AS module, loc.name AS name, loc.arity AS arity,
     count(loc) AS clauses,
     min(loc.start_line) AS first_line,
     max(loc.end_line) AS last_line,
     collect(loc.file)[0] AS file,
     collect(loc.generated_by)[0] AS generated_by
WHERE clauses >= $min_clauses
RETURN module, name, arity, clauses, first_line, last_line, file, generated_by
ORDER BY clauses DESC, module, name
LIMIT {}"#,
            self.limit
        ))
    }
}

pub fn find_many_clauses(
    db: &dyn DatabaseBackend,
    min_clauses: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<ManyClauses>, Box<dyn Error>> {
    let builder = ManyClausesQueryBuilder {
        min_clauses,
        module_pattern: module_pattern.map(|s| s.to_string()),
        project: project.to_string(),
        use_regex,
        include_generated,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| ManyClausesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 8 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let clauses = extract_i64(&row[3], 0);
            let first_line = extract_i64(&row[4], 0);
            let last_line = extract_i64(&row[5], 0);
            let Some(file) = extract_string(&row[6]) else { continue };
            let Some(generated_by) = extract_string(&row[7]) else { continue };

            results.push(ManyClauses {
                module,
                name,
                arity,
                clauses,
                first_line,
                last_line,
                file,
                generated_by,
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
    fn test_many_clauses_query_cozo_basic() {
        let builder = ManyClausesQueryBuilder {
            min_clauses: 5,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Verify aggregation pattern
        assert!(compiled.contains("clause_counts["));
        assert!(compiled.contains("count(line)"));
        assert!(compiled.contains("min(start_line)"));
        assert!(compiled.contains("max(end_line)"));
        assert!(compiled.contains("clauses >= $min_clauses"));
    }

    #[test]
    fn test_many_clauses_query_cozo_include_generated() {
        let builder = ManyClausesQueryBuilder {
            min_clauses: 3,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: true,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // When including generated, no filter should be present
        assert!(!compiled.contains("generated_by == \"\""));
    }

    #[test]
    fn test_many_clauses_query_cozo_exclude_generated() {
        let builder = ManyClausesQueryBuilder {
            min_clauses: 3,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("generated_by == \"\""));
    }

    #[test]
    fn test_many_clauses_query_cozo_with_module_pattern() {
        let builder = ManyClausesQueryBuilder {
            min_clauses: 2,
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            include_generated: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_many_clauses_query_age() {
        let builder = ManyClausesQueryBuilder {
            min_clauses: 5,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (loc:FunctionLocation)"));
        assert!(compiled.contains("count(loc)"));
        assert!(compiled.contains("min(loc.start_line)"));
        assert!(compiled.contains("max(loc.end_line)"));
    }

    #[test]
    fn test_many_clauses_query_parameters() {
        let builder = ManyClausesQueryBuilder {
            min_clauses: 10,
            module_pattern: Some("test".to_string()),
            project: "proj".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 3);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("min_clauses"));
        assert!(params.contains_key("module_pattern"));
    }
}
