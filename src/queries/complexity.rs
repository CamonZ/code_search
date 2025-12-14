use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum ComplexityError {
    #[error("Complexity query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with complexity metrics
#[derive(Debug, Clone, Serialize)]
pub struct ComplexityMetric {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub complexity: i64,
    pub max_nesting_depth: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub lines: i64,
    pub generated_by: String,
}

/// Query builder for finding functions with complexity metrics
#[derive(Debug)]
pub struct ComplexityQueryBuilder {
    pub min_complexity: i64,
    pub min_depth: i64,
    pub module_pattern: Option<String>,
    pub project: String,
    pub use_regex: bool,
    pub exclude_generated: bool,
    pub limit: u32,
}

impl QueryBuilder for ComplexityQueryBuilder {
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
        params.insert("min_complexity".to_string(), DataValue::from(self.min_complexity));
        params.insert("min_depth".to_string(), DataValue::from(self.min_depth));
        if let Some(ref pattern) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(pattern.clone().into()));
        }
        params
    }
}

impl ComplexityQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build optional module filter
        let module_filter = match &self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", str_includes(module, $module_pattern)".to_string(),
            None => String::new(),
        };

        // Build optional generated filter
        let generated_filter = if self.exclude_generated {
            ", generated_by == \"\"".to_string()
        } else {
            String::new()
        };

        Ok(format!(
            r#"?[module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, lines, generated_by] :=
    *function_locations{{project, module, name, arity, line, complexity, max_nesting_depth, start_line, end_line, generated_by}},
    project == $project,
    complexity >= $min_complexity,
    max_nesting_depth >= $min_depth,
    lines = end_line - start_line + 1
    {module_filter}
    {generated_filter}

:order -complexity, module, name
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };

        // Build WHERE conditions
        let mut where_conditions = vec![
            "f.project = $project".to_string(),
            format!("loc.complexity >= $min_complexity"),
            format!("loc.max_nesting_depth >= $min_depth"),
        ];

        // Add module filter if present
        if self.module_pattern.is_some() {
            where_conditions.push(format!("f.module {} $module_pattern", mod_match));
        }

        // Add generated filter if needed
        if self.exclude_generated {
            where_conditions.push("loc.generated_by = ''".to_string());
        }

        let where_clause = where_conditions.join("\n  AND ");

        Ok(format!(
            r#"MATCH (f:Function)-[:DEFINED_IN]->(loc:FunctionLocation)
WHERE {where_clause}
WITH f.module as module, f.name as name, f.arity as arity,
     loc.line as line, loc.complexity as complexity,
     loc.max_nesting_depth as max_nesting_depth,
     loc.start_line as start_line, loc.end_line as end_line,
     loc.end_line - loc.start_line + 1 as lines,
     loc.generated_by as generated_by
ORDER BY complexity DESC, module, name
LIMIT {}
RETURN module, name, arity, line, complexity, max_nesting_depth,
       start_line, end_line, lines, generated_by"#,
            self.limit
        ))
    }
}

pub fn find_complexity_metrics(
    db: &dyn DatabaseBackend,
    min_complexity: i64,
    min_depth: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<ComplexityMetric>, Box<dyn Error>> {
    let builder = ComplexityQueryBuilder {
        min_complexity,
        min_depth,
        module_pattern: module_pattern.map(|s| s.to_string()),
        project: project.to_string(),
        use_regex,
        exclude_generated,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| ComplexityError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 10 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let line = extract_i64(&row[3], 0);
            let complexity = extract_i64(&row[4], 0);
            let max_nesting_depth = extract_i64(&row[5], 0);
            let start_line = extract_i64(&row[6], 0);
            let end_line = extract_i64(&row[7], 0);
            let lines = extract_i64(&row[8], 0);
            let Some(generated_by) = extract_string(&row[9]) else { continue };

            results.push(ComplexityMetric {
                module,
                name,
                arity,
                line,
                complexity,
                max_nesting_depth,
                start_line,
                end_line,
                lines,
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
    fn test_complexity_query_cozo_basic() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 5,
            min_depth: 2,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            exclude_generated: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("*function_locations"));
        assert!(compiled.contains("complexity >= $min_complexity"));
        assert!(compiled.contains("max_nesting_depth >= $min_depth"));
        assert!(compiled.contains(":order -complexity"));
    }

    #[test]
    fn test_complexity_query_cozo_exclude_generated() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 1,
            min_depth: 0,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            exclude_generated: true,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("generated_by == \"\""));
    }

    #[test]
    fn test_complexity_query_cozo_with_module_pattern() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 3,
            min_depth: 1,
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            exclude_generated: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_complexity_query_age() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 5,
            min_depth: 2,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            exclude_generated: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH"));
        assert!(compiled.contains("complexity"));
        assert!(compiled.contains("max_nesting_depth"));
        assert!(compiled.contains("ORDER BY"));
    }

    #[test]
    fn test_complexity_query_parameters() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 10,
            min_depth: 3,
            module_pattern: Some("test".to_string()),
            project: "proj".to_string(),
            use_regex: false,
            exclude_generated: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 4);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("min_complexity"));
        assert!(params.contains_key("min_depth"));
        assert!(params.contains_key("module_pattern"));
    }

    #[test]
    fn test_complexity_query_parameters_without_module() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 10,
            min_depth: 3,
            module_pattern: None,
            project: "proj".to_string(),
            use_regex: false,
            exclude_generated: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 3);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("min_complexity"));
        assert!(params.contains_key("min_depth"));
        assert!(!params.contains_key("module_pattern"));
    }

    #[test]
    fn test_complexity_query_age_with_module_pattern() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 5,
            min_depth: 2,
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            exclude_generated: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("f.module =~"));
    }

    #[test]
    fn test_complexity_query_age_exclude_generated() {
        let builder = ComplexityQueryBuilder {
            min_complexity: 5,
            min_depth: 2,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            exclude_generated: true,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("loc.generated_by = ''"));
    }
}
