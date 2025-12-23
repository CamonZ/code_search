//! SurrealDB backend implementation.
//!
//! This module provides a stub implementation of the SurrealDB backend
//! that compiles but returns `unimplemented!()` for all operations.
//! The actual implementation will be completed in Phase 2.

use super::{Database, QueryParams, QueryResult};
use std::error::Error;
use std::path::Path;

/// SurrealDB backend implementation
///
/// TODO: Full implementation in Phase 2
/// This is a stub to enable compilation with backend-surrealdb feature
#[allow(dead_code)]
pub struct SurrealDatabase {
    // TODO: Add surrealdb::Surreal<Db> field
}

impl SurrealDatabase {
    /// Opens a SurrealDB database at the specified path.
    ///
    /// # Panics
    /// This method is not yet implemented and will panic if called.
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let _ = path; // Suppress unused variable warning
        unimplemented!(
            "SurrealDB backend not yet implemented. \
             Use --features backend-cozo for working backend."
        )
    }

    /// Opens an in-memory SurrealDB database for testing.
    ///
    /// # Panics
    /// This method is not yet implemented and will panic if called.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn open_mem() -> Self {
        unimplemented!("SurrealDB in-memory database not yet implemented")
    }
}

impl Database for SurrealDatabase {
    fn execute_query(
        &self,
        _query: &str,
        _params: QueryParams,
    ) -> Result<Box<dyn QueryResult>, Box<dyn Error>> {
        unimplemented!("SurrealDB query execution not yet implemented")
    }
}

// TODO: Implement SurrealQueryResult, SurrealRow, Value for SurrealDB types
// These will be added in Phase 2 when SurrealDB schema is defined
