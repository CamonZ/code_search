//! Query execution utilities.

use std::collections::BTreeMap;
use std::error::Error;

use cozo::{DataValue, NamedRows};

use super::DatabaseBackend;

/// Type alias for query parameters.
pub type Params = BTreeMap<String, DataValue>;

/// Run a query with parameters, supporting both Cozo and PostgreSQL/AGE backends.
///
/// This function:
/// 1. Compiles the query to the appropriate backend dialect via the QueryBuilder
/// 2. Executes the query using the backend's execute_query() method
/// 3. Converts the result to NamedRows for compatibility with existing code
pub fn run_query(
    db: &dyn DatabaseBackend,
    script: &str,
    params: Params,
) -> Result<NamedRows, Box<dyn Error>> {
    let query_result = db.execute_query(script, &params)?;
    Ok(NamedRows {
        headers: query_result.headers,
        rows: query_result.rows,
        next: None,
    })
}

/// Run a mutable query with no parameters
pub fn run_query_no_params(db: &dyn DatabaseBackend, script: &str) -> Result<NamedRows, Box<dyn Error>> {
    run_query(db, script, Params::new())
}

/// Try to create a relation, returning Ok(true) if created, Ok(false) if already exists
pub fn try_create_relation(db: &dyn DatabaseBackend, script: &str) -> Result<bool, Box<dyn Error>> {
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
