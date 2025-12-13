//! Query execution utilities.

use std::collections::BTreeMap;
use std::error::Error;

use cozo::{DataValue, DbInstance, NamedRows, ScriptMutability};

use super::DbError;

/// Type alias for query parameters.
pub type Params = BTreeMap<String, DataValue>;

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
