//! Database connection management.

use std::error::Error;
use std::path::Path;

use cozo::DbInstance;

use super::DbError;

/// Open a CozoDB database backed by SQLite storage.
pub fn open_db(path: &Path) -> Result<DbInstance, Box<dyn Error>> {
    DbInstance::new("sqlite", path, "").map_err(|e| {
        Box::new(DbError::OpenFailed {
            path: path.display().to_string(),
            message: format!("{:?}", e),
        }) as Box<dyn Error>
    })
}

/// Create an in-memory database instance.
///
/// Used for tests to avoid disk I/O and temp file management.
#[cfg(test)]
pub fn open_mem_db() -> DbInstance {
    DbInstance::new("mem", "", "").expect("Failed to create in-memory DB")
}
