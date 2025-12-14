use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum FileError {
    #[error("File query failed: {message}")]
    QueryFailed { message: String },
}

/// A function defined in a file
#[derive(Debug, Clone, Serialize)]
pub struct FileFunctionDef {
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    pub pattern: String,
    pub guard: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub file: String,
}

/// Query builder for finding functions in a module
#[derive(Debug)]
pub struct FileQueryBuilder {
    pub module_pattern: String,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for FileQueryBuilder {
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
        params
    }
}

impl FileQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let module_filter = if self.use_regex {
            "regex_matches(module, $module_pattern)"
        } else {
            "module == $module_pattern"
        };

        Ok(format!(
            r#"?[module, name, arity, kind, line, start_line, end_line, file, pattern, guard] :=
    *function_locations{{project, module, name, arity, line, file, kind, start_line, end_line, pattern, guard}},
    project == $project,
    {module_filter}
:order module, start_line, name, arity, line
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };

        Ok(format!(
            r#"MATCH (fl:FunctionLocation)
WHERE fl.project = $project AND fl.module {} $module_pattern
RETURN fl.module, fl.name, fl.arity, fl.kind, fl.line,
       fl.start_line, fl.end_line, fl.file, fl.pattern, fl.guard
ORDER BY fl.module, fl.start_line, fl.name, fl.arity, fl.line
LIMIT {}"#,
            mod_match, self.limit
        ))
    }
}

/// Find all functions in modules matching a pattern
/// Returns a flat vec of functions with location info (for browse-module)
pub fn find_functions_in_module(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FileFunctionDef>, Box<dyn Error>> {
    let builder = FileQueryBuilder {
        module_pattern: module_pattern.to_string(),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| FileError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();

    for row in rows.rows {
        if row.len() >= 10 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let arity = extract_i64(&row[2], 0);
            let Some(kind) = extract_string(&row[3]) else { continue };
            let line = extract_i64(&row[4], 0);
            let start_line = extract_i64(&row[5], 0);
            let end_line = extract_i64(&row[6], 0);
            let file = extract_string(&row[7]).unwrap_or_default();
            let pattern = extract_string(&row[8]).unwrap_or_default();
            let guard = extract_string(&row[9]).unwrap_or_default();

            results.push(FileFunctionDef {
                module,
                name,
                arity,
                kind,
                line,
                start_line,
                end_line,
                pattern,
                guard,
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
    fn test_file_query_cozo_exact_match() {
        let builder = FileQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 1000,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("?[module, name, arity, kind, line, start_line, end_line, file, pattern, guard]"));
        assert!(compiled.contains("*function_locations"));
        assert!(compiled.contains("module == $module_pattern"));
        assert!(compiled.contains(":order module, start_line, name, arity, line"));
        assert!(compiled.contains(":limit 1000"));
    }

    #[test]
    fn test_file_query_cozo_regex() {
        let builder = FileQueryBuilder {
            module_pattern: "MyApp\\..*".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 500,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(module, $module_pattern)"));
    }

    #[test]
    fn test_file_query_age_exact_match() {
        let builder = FileQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 1000,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH (fl:FunctionLocation)"));
        assert!(compiled.contains("fl.module = $module_pattern"));
        assert!(compiled.contains("fl.project = $project"));
        assert!(compiled.contains("RETURN fl.module, fl.name, fl.arity"));
        assert!(compiled.contains("ORDER BY fl.module, fl.start_line, fl.name, fl.arity, fl.line"));
        assert!(compiled.contains("LIMIT 1000"));
    }

    #[test]
    fn test_file_query_age_regex() {
        let builder = FileQueryBuilder {
            module_pattern: "MyApp\\..*".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 500,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("fl.module =~ $module_pattern"));
    }

    #[test]
    fn test_file_query_parameters() {
        let builder = FileQueryBuilder {
            module_pattern: "Module".to_string(),
            project: "proj".to_string(),
            use_regex: false,
            limit: 100,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("module_pattern"));
        assert!(params.contains_key("project"));
    }

    #[test]
    fn test_file_query_output_order() {
        // Verify the output fields are in the expected order for extraction
        let builder = FileQueryBuilder {
            module_pattern: "Test".to_string(),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Fields should be in this specific order for extraction:
        // module, name, arity, kind, line, start_line, end_line, file, pattern, guard
        assert!(compiled.contains("?[module, name, arity, kind, line, start_line, end_line, file, pattern, guard]"));
    }
}
