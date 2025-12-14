use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum TypesError {
    #[error("Types query failed: {message}")]
    QueryFailed { message: String },
}

/// A type definition (@type, @typep, @opaque)
#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    pub project: String,
    pub module: String,
    pub name: String,
    pub kind: String,
    pub params: String,
    pub line: i64,
    pub definition: String,
}

/// Query builder for finding types by module pattern with optional name and kind filters
#[derive(Debug)]
pub struct TypesQueryBuilder {
    pub module_pattern: String,
    pub name_filter: Option<String>,
    pub kind_filter: Option<String>,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for TypesQueryBuilder {
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

        if let Some(ref name) = self.name_filter {
            params.insert("name_pattern".to_string(), DataValue::Str(name.clone().into()));
        }

        if let Some(ref kind) = self.kind_filter {
            params.insert("kind".to_string(), DataValue::Str(kind.clone().into()));
        }
        params
    }
}

impl TypesQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build module filter
        let module_filter = if self.use_regex {
            "regex_matches(module, $module_pattern)"
        } else {
            "module == $module_pattern"
        };

        // Build name filter
        let name_filter_sql = match &self.name_filter {
            Some(_) if self.use_regex => ", regex_matches(name, $name_pattern)",
            Some(_) => ", str_includes(name, $name_pattern)",
            None => "",
        };

        // Build kind filter
        let kind_filter_sql = match &self.kind_filter {
            Some(_) => ", kind == $kind",
            None => "",
        };

        Ok(format!(
            r#"?[project, module, name, kind, params, line, definition] :=
    *types{{project, module, name, kind, params, line, definition}},
    project == $project,
    {module_filter}
    {name_filter_sql}
    {kind_filter_sql}
:order module, name
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };

        let mut conditions = vec![
            "t.project = $project".to_string(),
            format!("t.module {} $module_pattern", mod_match),
        ];

        if self.name_filter.is_some() {
            let name_match = if self.use_regex { "=~" } else { "CONTAINS" };
            conditions.push(format!("t.name {} $name_pattern", name_match));
        }

        if self.kind_filter.is_some() {
            conditions.push("t.kind = $kind".to_string());
        }

        Ok(format!(
            r#"MATCH (t:Type)
WHERE {}
RETURN t.project, t.module, t.name, t.kind, t.params, t.line, t.definition
ORDER BY t.module, t.name
LIMIT {}"#,
            conditions.join(" AND "),
            self.limit
        ))
    }
}

pub fn find_types(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    name_filter: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<TypeInfo>, Box<dyn Error>> {
    let builder = TypesQueryBuilder {
        module_pattern: module_pattern.to_string(),
        name_filter: name_filter.map(String::from),
        kind_filter: kind_filter.map(String::from),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| TypesError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 7 {
            let Some(project) = extract_string(&row[0]) else {
                continue;
            };
            let Some(module) = extract_string(&row[1]) else {
                continue;
            };
            let Some(name) = extract_string(&row[2]) else {
                continue;
            };
            let Some(kind) = extract_string(&row[3]) else {
                continue;
            };
            let params_str = extract_string(&row[4]).unwrap_or_default();
            let line = extract_i64(&row[5], 0);
            let definition = extract_string(&row[6]).unwrap_or_default();

            results.push(TypeInfo {
                project,
                module,
                name,
                kind,
                params: params_str,
                line,
                definition,
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
    fn test_types_query_cozo_basic() {
        let builder = TypesQueryBuilder {
            module_pattern: "MyApp.Types".to_string(),
            name_filter: None,
            kind_filter: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("?[project, module, name, kind, params, line, definition]"));
        assert!(compiled.contains("*types"));
        assert!(compiled.contains("module == $module_pattern"));
        assert!(compiled.contains(":order module, name"));
    }

    #[test]
    fn test_types_query_cozo_with_name_filter() {
        let builder = TypesQueryBuilder {
            module_pattern: "MyApp".to_string(),
            name_filter: Some("user".to_string()),
            kind_filter: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("str_includes(name, $name_pattern)"));
    }

    #[test]
    fn test_types_query_cozo_with_kind_filter() {
        let builder = TypesQueryBuilder {
            module_pattern: "MyApp".to_string(),
            name_filter: None,
            kind_filter: Some("opaque".to_string()),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("kind == $kind"));
    }

    #[test]
    fn test_types_query_cozo_regex_all_filters() {
        let builder = TypesQueryBuilder {
            module_pattern: "MyApp\\..*".to_string(),
            name_filter: Some(".*_t$".to_string()),
            kind_filter: Some("type".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(module, $module_pattern)"));
        assert!(compiled.contains("regex_matches(name, $name_pattern)"));
        assert!(compiled.contains("kind == $kind"));
    }

    #[test]
    fn test_types_query_age_basic() {
        let builder = TypesQueryBuilder {
            module_pattern: "MyApp.Types".to_string(),
            name_filter: None,
            kind_filter: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH (t:Type)"));
        assert!(compiled.contains("t.module = $module_pattern"));
        assert!(compiled.contains("t.project = $project"));
        assert!(compiled.contains("RETURN t.project, t.module, t.name"));
        assert!(compiled.contains("ORDER BY t.module, t.name"));
    }

    #[test]
    fn test_types_query_age_all_filters() {
        let builder = TypesQueryBuilder {
            module_pattern: "MyApp".to_string(),
            name_filter: Some("socket".to_string()),
            kind_filter: Some("opaque".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 25,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("t.module =~ $module_pattern"));
        assert!(compiled.contains("t.name =~ $name_pattern"));
        assert!(compiled.contains("t.kind = $kind"));
        assert!(compiled.contains("LIMIT 25"));
    }

    #[test]
    fn test_types_query_parameters_minimal() {
        let builder = TypesQueryBuilder {
            module_pattern: "mod".to_string(),
            name_filter: None,
            kind_filter: None,
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2); // project, module_pattern
    }

    #[test]
    fn test_types_query_parameters_full() {
        let builder = TypesQueryBuilder {
            module_pattern: "mod".to_string(),
            name_filter: Some("name".to_string()),
            kind_filter: Some("type".to_string()),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 4);
        assert!(params.contains_key("name_pattern"));
        assert!(params.contains_key("kind"));
    }
}
