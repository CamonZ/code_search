use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum DuplicatesError {
    #[error("Duplicates query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that has a duplicate implementation (same AST or source hash)
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateFunction {
    pub hash: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub line: i64,
    pub file: String,
}

/// Query builder for finding duplicate functions
#[derive(Debug)]
pub struct DuplicatesQueryBuilder {
    pub project: String,
    pub module_pattern: Option<String>,
    pub use_regex: bool,
    pub use_exact: bool,  // true = source_sha, false = ast_sha
}

impl QueryBuilder for DuplicatesQueryBuilder {
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

impl DuplicatesQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Choose hash field based on exact flag
        let hash_field = if self.use_exact { "source_sha" } else { "ast_sha" };

        // Build optional module filter
        let module_filter = match self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", str_includes(module, $module_pattern)".to_string(),
            None => String::new(),
        };

        Ok(format!(
            r#"# Find hashes that appear more than once (count unique functions per hash)
hash_counts[{hash_field}, count(module)] :=
    *function_locations{{project, module, name, arity, {hash_field}}},
    project == $project,
    {hash_field} != ""

# Get all functions with duplicate hashes
?[{hash_field}, module, name, arity, line, file] :=
    *function_locations{{project, module, name, arity, line, file, {hash_field}}},
    hash_counts[{hash_field}, cnt],
    cnt > 1,
    project == $project
    {module_filter}

:order {hash_field}, module, name, arity"#,
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices only, not edges.
        // FunctionLocation has: module, name, arity, source_sha, ast_sha, line, file

        // Choose hash field based on exact flag
        let hash_field = if self.use_exact { "source_sha" } else { "ast_sha" };

        let mod_match = if self.use_regex { "=~" } else { "=" };

        let where_filter = match &self.module_pattern {
            Some(_) => format!("\n  AND loc2.module {} $module_pattern", mod_match),
            None => String::new(),
        };

        // AGE doesn't support multi-statement queries, so we need to use subquery or collect
        // Using a WITH pattern to first find duplicate hashes
        Ok(format!(
            r#"MATCH (loc:FunctionLocation)
WHERE loc.project = $project
  AND loc.{hash_field} <> ''
WITH loc.{hash_field} AS hash, count(loc) AS cnt
WHERE cnt > 1
MATCH (loc2:FunctionLocation)
WHERE loc2.project = $project
  AND loc2.{hash_field} = hash{where_filter}
RETURN loc2.{hash_field} AS hash, loc2.module, loc2.name, loc2.arity, loc2.line, loc2.file
ORDER BY hash, loc2.module, loc2.name, loc2.arity"#,
        ))
    }
}

pub fn find_duplicates(
    db: &dyn DatabaseBackend,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
    use_exact: bool,
) -> Result<Vec<DuplicateFunction>, Box<dyn Error>> {
    let builder = DuplicatesQueryBuilder {
        project: project.to_string(),
        module_pattern: module_pattern.map(|s| s.to_string()),
        use_regex,
        use_exact,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| DuplicatesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(hash) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let line = extract_i64(&row[4], 0);
            let Some(file) = extract_string(&row[5]) else { continue };

            results.push(DuplicateFunction {
                hash,
                module,
                name,
                arity,
                line,
                file,
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
    fn test_duplicates_query_cozo_ast_sha() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
            use_regex: false,
            use_exact: false,  // AST hash
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("ast_sha"));
        assert!(compiled.contains("hash_counts"));
        assert!(compiled.contains("cnt > 1"));
    }

    #[test]
    fn test_duplicates_query_cozo_source_sha() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
            use_regex: false,
            use_exact: true,  // Source hash
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("source_sha"));
        assert!(compiled.contains("hash_counts"));
    }

    #[test]
    fn test_duplicates_query_cozo_with_module_pattern() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: Some("MyApp".to_string()),
            use_regex: true,
            use_exact: false,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_duplicates_query_cozo_with_module_pattern_str_includes() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: Some("MyApp".to_string()),
            use_regex: false,
            use_exact: false,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("str_includes"));
    }

    #[test]
    fn test_duplicates_query_age() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
            use_regex: false,
            use_exact: false,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (loc:FunctionLocation)"));
        assert!(compiled.contains("count(loc)"));
        assert!(compiled.contains("cnt > 1"));
    }

    #[test]
    fn test_duplicates_query_age_with_module_pattern() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: Some("TestModule".to_string()),
            use_regex: false,
            use_exact: false,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (loc:FunctionLocation)"));
        assert!(compiled.contains("loc2.module ="));
    }

    #[test]
    fn test_duplicates_query_age_with_module_pattern_regex() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: Some("Test.*".to_string()),
            use_regex: true,
            use_exact: false,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("loc2.module =~"));
    }

    #[test]
    fn test_duplicates_query_age_source_sha() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
            use_regex: false,
            use_exact: true,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("source_sha"));
    }

    #[test]
    fn test_duplicates_query_parameters() {
        let builder = DuplicatesQueryBuilder {
            project: "proj".to_string(),
            module_pattern: Some("test".to_string()),
            use_regex: false,
            use_exact: false,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("module_pattern"));
    }

    #[test]
    fn test_duplicates_query_parameters_no_pattern() {
        let builder = DuplicatesQueryBuilder {
            project: "proj".to_string(),
            module_pattern: None,
            use_regex: false,
            use_exact: false,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 1);
        assert!(params.contains_key("project"));
    }

    #[test]
    fn test_duplicates_query_ordering() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
            use_regex: false,
            use_exact: false,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Check that ordering is by hash, module, name, arity
        assert!(compiled.contains(":order"));
    }

    #[test]
    fn test_duplicates_query_filters_empty_hashes() {
        let builder = DuplicatesQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
            use_regex: false,
            use_exact: false,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Check that empty hashes are filtered
        assert!(compiled.contains("!= \"\""));
    }
}
