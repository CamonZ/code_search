//! Query to get all module calls for cluster analysis.
//!
//! Returns calls between different modules (no self-calls).
//! Clusters are computed in Rust by grouping modules by namespace.

use std::error::Error;

use crate::backend::{Database, QueryParams};
use crate::db::run_query;

/// Represents a call between two different modules
#[derive(Debug, Clone)]
pub struct ModuleCall {
    pub caller_module: String,
    pub callee_module: String,
}

/// Get all inter-module calls (calls between different modules)
///
/// Returns calls where caller_module != callee_module.
/// These are used to compute internal vs external connectivity per namespace cluster.
pub fn get_module_calls(db: &dyn Database, project: &str) -> Result<Vec<ModuleCall>, Box<dyn Error>> {
    let script = r#"
        ?[caller_module, callee_module] :=
            *calls{project, caller_module, callee_module},
            project == $project,
            caller_module != callee_module
    "#;

    let params = QueryParams::new()
        .with_str("project", project);

    let result = run_query(db, script, params)?;

    let caller_idx = result.headers().iter().position(|h| h == "caller_module")
        .ok_or("Missing caller_module column")?;
    let callee_idx = result.headers().iter().position(|h| h == "callee_module")
        .ok_or("Missing callee_module column")?;

    let results = result
        .rows()
        .iter()
        .filter_map(|row| {
            let caller = row.get(caller_idx).and_then(|v| v.as_str());
            let callee = row.get(callee_idx).and_then(|v| v.as_str());
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
