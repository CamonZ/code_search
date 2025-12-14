use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum LargeFunctionsError {
    #[error("Large functions query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with line count information
#[derive(Debug, Clone, Serialize)]
pub struct LargeFunction {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub lines: i64,
    pub file: String,
    pub generated_by: String,
}

/// Query builder for finding functions larger than a minimum line count
#[derive(Debug)]
pub struct LargeFunctionsQueryBuilder {
    pub min_lines: i64,
    pub module_pattern: Option<String>,
    pub project: String,
    pub use_regex: bool,
    pub include_generated: bool,
    pub limit: u32,
}

impl QueryBuilder for LargeFunctionsQueryBuilder {
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
        params.insert("min_lines".to_string(), DataValue::from(self.min_lines));
        if let Some(ref pattern) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(pattern.clone().into()));
        }
        params
    }
}

impl LargeFunctionsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build optional module filter
        let module_filter = match &self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", str_includes(module, $module_pattern)".to_string(),
            None => String::new(),
        };

        // Build optional generated filter
        let generated_filter = if self.include_generated {
            String::new()
        } else {
            ", generated_by == \"\"".to_string()
        };

        Ok(format!(
            r#"?[module, name, arity, start_line, end_line, lines, file, generated_by] :=
    *function_locations{{project, module, name, arity, line, start_line, end_line, file, generated_by}},
    project == $project,
    lines = end_line - start_line + 1,
    lines >= $min_lines
    {module_filter}
    {generated_filter}

:order -lines, module, name
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };

        // Build WHERE conditions
        let mut where_conditions = vec![
            "f.project = $project".to_string(),
            "(loc.end_line - loc.start_line + 1) >= $min_lines".to_string(),
        ];

        // Add module filter if present
        if self.module_pattern.is_some() {
            where_conditions.push(format!("f.module {} $module_pattern", mod_match));
        }

        // Add generated filter if needed
        if !self.include_generated {
            where_conditions.push("loc.generated_by = ''".to_string());
        }

        let where_clause = where_conditions.join("\n  AND ");

        Ok(format!(
            r#"MATCH (f:Function)-[:DEFINED_IN]->(loc:FunctionLocation)
WHERE {where_clause}
WITH f.module as module, f.name as name, f.arity as arity,
     loc.start_line as start_line, loc.end_line as end_line,
     loc.end_line - loc.start_line + 1 as lines,
     loc.file as file, loc.generated_by as generated_by
ORDER BY lines DESC, module, name
LIMIT {}
RETURN module, name, arity, start_line, end_line, lines, file, generated_by"#,
            self.limit
        ))
    }
}

pub fn find_large_functions(
    db: &dyn DatabaseBackend,
    min_lines: i64,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    include_generated: bool,
    limit: u32,
) -> Result<Vec<LargeFunction>, Box<dyn Error>> {
    let builder = LargeFunctionsQueryBuilder {
        min_lines,
        module_pattern: module_pattern.map(|s| s.to_string()),
        project: project.to_string(),
        use_regex,
        include_generated,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| LargeFunctionsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 8 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let start_line = extract_i64(&row[3], 0);
            let end_line = extract_i64(&row[4], 0);
            let lines = extract_i64(&row[5], 0);
            let Some(file) = extract_string(&row[6]) else { continue };
            let Some(generated_by) = extract_string(&row[7]) else { continue };

            results.push(LargeFunction {
                module,
                name,
                arity,
                start_line,
                end_line,
                lines,
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
    fn test_large_functions_query_cozo_basic() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 50,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("*function_locations"));
        assert!(compiled.contains("lines = end_line - start_line + 1"));
        assert!(compiled.contains("lines >= $min_lines"));
        assert!(compiled.contains(":order -lines"));
    }

    #[test]
    fn test_large_functions_query_cozo_include_generated() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 20,
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
    fn test_large_functions_query_cozo_exclude_generated() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 20,
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
    fn test_large_functions_query_cozo_with_module_pattern() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 30,
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
    fn test_large_functions_query_age() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 50,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH"));
        assert!(compiled.contains("end_line - loc.start_line + 1"));
        assert!(compiled.contains("ORDER BY lines DESC"));
    }

    #[test]
    fn test_large_functions_query_parameters() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 100,
            module_pattern: Some("test".to_string()),
            project: "proj".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 3);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("min_lines"));
        assert!(params.contains_key("module_pattern"));
    }

    #[test]
    fn test_large_functions_query_parameters_without_module() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 100,
            module_pattern: None,
            project: "proj".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("min_lines"));
        assert!(!params.contains_key("module_pattern"));
    }

    #[test]
    fn test_large_functions_query_age_with_module_pattern() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 30,
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            include_generated: false,
            limit: 50,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("f.module =~"));
    }

    #[test]
    fn test_large_functions_query_age_exclude_generated() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 50,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("loc.generated_by = ''"));
    }

    #[test]
    fn test_large_functions_query_age_include_generated() {
        let builder = LargeFunctionsQueryBuilder {
            min_lines: 50,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            include_generated: true,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // When including generated, no generated_by filter should be present
        assert!(!compiled.contains("loc.generated_by = ''"));
    }
}
