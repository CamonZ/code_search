use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use cozo::{DataValue, DbInstance, NamedRows, ScriptMutability};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Failed to open database '{path}': {message}")]
    OpenFailed { path: String, message: String },

    #[error("Query failed: {message}")]
    QueryFailed { message: String },
}

pub type Params = BTreeMap<String, DataValue>;

pub fn open_db(path: &Path) -> Result<DbInstance, Box<dyn Error>> {
    DbInstance::new("sqlite", path, "").map_err(|e| {
        Box::new(DbError::OpenFailed {
            path: path.display().to_string(),
            message: format!("{:?}", e),
        }) as Box<dyn Error>
    })
}

/// Run a mutable query (insert, delete, create, etc.)
pub fn run_query(
    db: &DbInstance,
    script: &str,
    params: Params,
) -> Result<NamedRows, Box<dyn Error>> {
    db.run_script(script, params, ScriptMutability::Mutable)
        .map_err(|e| {
            Box::new(DbError::QueryFailed {
                message: format!("{:?}", e),
            }) as Box<dyn Error>
        })
}

/// Run a mutable query with no parameters
pub fn run_query_no_params(db: &DbInstance, script: &str) -> Result<NamedRows, Box<dyn Error>> {
    run_query(db, script, Params::new())
}

/// Escape a string for use in CozoDB string literals
pub fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Try to create a relation, returning Ok(true) if created, Ok(false) if already exists
pub fn try_create_relation(db: &DbInstance, script: &str) -> Result<bool, Box<dyn Error>> {
    match run_query_no_params(db, script) {
        Ok(_) => Ok(true),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("AlreadyExists") || err_str.contains("stored_relation_conflict") {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}
