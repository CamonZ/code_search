//! Query to get all module calls for cluster analysis.
//!
//! Returns calls between different modules (no self-calls).
//! Clusters are computed in Rust by grouping modules by namespace.

use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;

use crate::db::{run_query, Params};
use crate::queries::builder::QueryBuilder;

/// Represents a call between two different modules
#[derive(Debug, Clone)]
pub struct ModuleCall {
    pub caller_module: String,
    pub callee_module: String,
}

/// Query builder for inter-module call queries
#[derive(Debug)]
pub struct ClustersQueryBuilder {
    pub project: String,
}

impl QueryBuilder for ClustersQueryBuilder {
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
        params
    }
}

impl ClustersQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        Ok(r#"?[caller_module, callee_module] :=
    *calls{project, caller_module, callee_module},
    project == $project,
    caller_module != callee_module"#
            .to_string())
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices only, not edges.
        // Call vertex has: caller_module, callee_module
        Ok(r#"MATCH (c:Call)
WHERE c.project = $project
  AND c.caller_module <> c.callee_module
RETURN DISTINCT c.caller_module AS caller_module, c.callee_module AS callee_module"#
            .to_string())
    }
}

/// Get all inter-module calls (calls between different modules)
///
/// Returns calls where caller_module != callee_module.
/// These are used to compute internal vs external connectivity per namespace cluster.
pub fn get_module_calls(db: &dyn DatabaseBackend, project: &str) -> Result<Vec<ModuleCall>, Box<dyn Error>> {
    use crate::queries::builder::CompiledQuery;

    let builder = ClustersQueryBuilder {
        project: project.to_string(),
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params)?;

    let caller_idx = rows.headers.iter().position(|h| h == "caller_module")
        .ok_or("Missing caller_module column")?;
    let callee_idx = rows.headers.iter().position(|h| h == "callee_module")
        .ok_or("Missing callee_module column")?;

    let results = rows
        .rows
        .iter()
        .filter_map(|row| {
            let caller = row.get(caller_idx).and_then(|v| v.get_str());
            let callee = row.get(callee_idx).and_then(|v| v.get_str());
            match (caller, callee) {
                (Some(c), Some(m)) => Some(ModuleCall {
                    caller_module: c.to_string(),
                    callee_module: m.to_string(),
                }),
                _ => None,
            }
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_clusters_query_cozo_basic() {
        let builder = ClustersQueryBuilder {
            project: "myproject".to_string(),
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("*calls"));
        assert!(compiled.contains("caller_module"));
        assert!(compiled.contains("callee_module"));
        assert!(compiled.contains("caller_module != callee_module"));
    }

    #[test]
    fn test_clusters_query_age() {
        let builder = ClustersQueryBuilder {
            project: "myproject".to_string(),
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (c:Call)"));
        assert!(compiled.contains("c.caller_module <> c.callee_module"));
    }

    #[test]
    fn test_clusters_query_parameters() {
        let builder = ClustersQueryBuilder {
            project: "proj".to_string(),
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 1);
        assert!(params.contains_key("project"));
    }
}
