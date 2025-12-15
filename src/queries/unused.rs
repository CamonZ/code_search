use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum UnusedError {
    #[error("Unused query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that is never called
#[derive(Debug, Clone, Serialize)]
pub struct UnusedFunction {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub file: String,
    pub line: i64,
}

/// Generated function name patterns to exclude (Elixir compiler-generated)
const GENERATED_PATTERNS: &[&str] = &[
    "__struct__",
    "__using__",
    "__before_compile__",
    "__after_compile__",
    "__on_definition__",
    "__impl__",
    "__info__",
    "__protocol__",
    "__deriving__",
    "__changeset__",
    "__schema__",
    "__meta__",
];

/// Query builder for finding unused functions
#[derive(Debug)]
pub struct UnusedQueryBuilder {
    pub module_pattern: Option<String>,
    pub project: String,
    pub use_regex: bool,
    pub private_only: bool,
    pub public_only: bool,
    pub limit: u32,
}

impl QueryBuilder for UnusedQueryBuilder {
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
        if let Some(ref pattern) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(pattern.clone().into()));
        }
        params
    }
}

impl UnusedQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let module_filter = match self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", str_includes(module, $module_pattern)".to_string(),
            None => String::new(),
        };

        let kind_filter = if self.private_only {
            ", (kind == \"defp\" or kind == \"defmacrop\")".to_string()
        } else if self.public_only {
            ", (kind == \"def\" or kind == \"defmacro\")".to_string()
        } else {
            String::new()
        };

        Ok(format!(
            r#"# All defined functions
defined[module, name, arity, kind, file, start_line] :=
    *function_locations{{project, module, name, arity, kind, file, start_line}},
    project == $project
    {module_filter}
    {kind_filter}

# All functions that are called (as callees)
called[module, name, arity] :=
    *calls{{project, callee_module, callee_function, callee_arity}},
    project == $project,
    module = callee_module,
    name = callee_function,
    arity = callee_arity

# Functions that are defined but never called
?[module, name, arity, kind, file, line] :=
    defined[module, name, arity, kind, file, line],
    not called[module, name, arity]

:order module, name, arity
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices only, not edges.
        // FunctionLocation has: module, name, arity, kind, file, start_line
        // Call has: callee_module, callee_function, callee_arity

        let mod_match = if self.use_regex { "=~" } else { "=" };

        let mut where_conditions = vec!["f.project = $project".to_string()];

        if let Some(_) = &self.module_pattern {
            where_conditions.push(format!("f.module {} $module_pattern", mod_match));
        }

        if self.private_only {
            where_conditions.push("(f.kind = 'defp' OR f.kind = 'defmacrop')".to_string());
        } else if self.public_only {
            where_conditions.push("(f.kind = 'def' OR f.kind = 'defmacro')".to_string());
        }

        let where_clause = where_conditions.join("\n  AND ");

        Ok(format!(
            r#"MATCH (f:FunctionLocation)
WHERE {where_clause}
OPTIONAL MATCH (c:Call)
WHERE c.project = $project
  AND c.callee_module = f.module
  AND c.callee_function STARTS WITH f.name
  AND c.callee_arity = f.arity
WITH f, count(c) AS call_count
WHERE call_count = 0
RETURN f.module, f.name, f.arity, f.kind, f.file, f.start_line AS line
ORDER BY f.module, f.name, f.arity
LIMIT {}"#,
            self.limit
        ))
    }
}

pub fn find_unused_functions(
    db: &dyn DatabaseBackend,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    private_only: bool,
    public_only: bool,
    exclude_generated: bool,
    limit: u32,
) -> Result<Vec<UnusedFunction>, Box<dyn Error>> {
    let builder = UnusedQueryBuilder {
        module_pattern: module_pattern.map(|s| s.to_string()),
        project: project.to_string(),
        use_regex,
        private_only,
        public_only,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| UnusedError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let Some(kind) = extract_string(&row[3]) else { continue };
            let Some(file) = extract_string(&row[4]) else { continue };
            let line = extract_i64(&row[5], 0);

            // Filter out generated functions if requested
            if exclude_generated && GENERATED_PATTERNS.iter().any(|p| name.starts_with(p)) {
                continue;
            }

            results.push(UnusedFunction {
                module,
                name,
                arity,
                kind,
                file,
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
    fn test_unused_query_cozo_basic() {
        let builder = UnusedQueryBuilder {
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            private_only: false,
            public_only: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Verify set difference pattern
        assert!(compiled.contains("defined["));
        assert!(compiled.contains("called["));
        assert!(compiled.contains("not called["));
    }

    #[test]
    fn test_unused_query_cozo_private_only() {
        let builder = UnusedQueryBuilder {
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            private_only: true,
            public_only: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("defp"));
        assert!(compiled.contains("defmacrop"));
    }

    #[test]
    fn test_unused_query_cozo_public_only() {
        let builder = UnusedQueryBuilder {
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            private_only: false,
            public_only: true,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("def"));
        assert!(compiled.contains("defmacro"));
    }

    #[test]
    fn test_unused_query_cozo_with_module_pattern() {
        let builder = UnusedQueryBuilder {
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            private_only: false,
            public_only: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_unused_query_cozo_with_module_pattern_literal() {
        let builder = UnusedQueryBuilder {
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: false,
            private_only: false,
            public_only: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("str_includes"));
    }

    #[test]
    fn test_unused_query_age() {
        let builder = UnusedQueryBuilder {
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            private_only: false,
            public_only: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (f:FunctionLocation)"));
        assert!(compiled.contains("OPTIONAL MATCH (c:Call)"));
        assert!(compiled.contains("c.callee_module = f.module"));
    }

    #[test]
    fn test_unused_query_age_with_module_pattern() {
        let builder = UnusedQueryBuilder {
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: false,
            private_only: false,
            public_only: false,
            limit: 50,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("f.module ="));
    }

    #[test]
    fn test_unused_query_age_with_module_pattern_regex() {
        let builder = UnusedQueryBuilder {
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            private_only: false,
            public_only: false,
            limit: 50,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("f.module =~"));
    }

    #[test]
    fn test_unused_query_age_private_only() {
        let builder = UnusedQueryBuilder {
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            private_only: true,
            public_only: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("defp"));
        assert!(compiled.contains("defmacrop"));
    }

    #[test]
    fn test_unused_query_age_public_only() {
        let builder = UnusedQueryBuilder {
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            private_only: false,
            public_only: true,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("f.kind = 'def'"));
        assert!(compiled.contains("f.kind = 'defmacro'"));
    }

    #[test]
    fn test_unused_query_parameters() {
        let builder = UnusedQueryBuilder {
            module_pattern: Some("test".to_string()),
            project: "proj".to_string(),
            use_regex: false,
            private_only: false,
            public_only: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("module_pattern"));
    }

    #[test]
    fn test_unused_query_parameters_no_pattern() {
        let builder = UnusedQueryBuilder {
            module_pattern: None,
            project: "proj".to_string(),
            use_regex: false,
            private_only: false,
            public_only: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 1);
        assert!(params.contains_key("project"));
        assert!(!params.contains_key("module_pattern"));
    }
}
