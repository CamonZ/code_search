//! PostgreSQL with Apache AGE backend implementation.
//!
//! This module provides a database backend using PostgreSQL with the Apache AGE
//! graph extension, enabling Cypher queries on a relational database.

mod conversion;
mod schema;

use std::sync::RwLock;
use std::error::Error;

use apache_age::sync::{AgeClient, Client};
use apache_age::NoTls;
use cozo::{DataValue, DbInstance};

use super::backend::{DatabaseBackend, Params, QueryResult};
use super::schema::SchemaRelation;

/// PostgreSQL backend using Apache AGE for graph queries.
///
/// Uses the `apache-age` crate which provides:
/// - Synchronous API via `sync` feature
/// - Type-safe Cypher queries via `query_cypher::<T>()`
/// - Serde integration with `AgType<T>`
pub struct PostgresAgeBackend {
    /// The database client (wrapped in RwLock for interior mutability)
    /// Note: Client is not Sync, but we manually implement Sync for PostgresAgeBackend
    /// because the Client is protected by RwLock and only accessed serially.
    client: RwLock<Client>,
    /// Name of the AGE graph to use
    graph_name: String,
}

// Manual Sync implementation: Client is guarded by RwLock and used serially
// so it's safe to share across threads
unsafe impl Sync for PostgresAgeBackend {}

impl PostgresAgeBackend {
    /// Create a new PostgreSQL AGE backend.
    ///
    /// # Arguments
    /// * `connection_string` - PostgreSQL connection string (e.g., "postgres://user:pass@host:port/db")
    /// * `graph_name` - Name of the AGE graph to use
    ///
    /// # Errors
    /// Returns an error if:
    /// - Connection fails
    /// - AGE extension is not installed
    ///
    /// # Note
    /// This constructor only connects to the database and verifies AGE is available.
    /// To create the graph and initialize the schema, use the `setup` command.
    ///
    /// # Example
    /// ```no_run
    /// let backend = PostgresAgeBackend::new(
    ///     "postgres://user:pass@localhost:5432/code_search",
    ///     "call_graph"
    /// )?;
    /// ```
    pub fn new(connection_string: &str, graph_name: &str) -> Result<Self, Box<dyn Error>> {
        // Connect using apache-age's sync client
        let mut client = Client::connect_age(connection_string, NoTls)
            .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;

        // Verify AGE is available by checking if we can query it
        Self::verify_age_extension(&mut client)?;

        Ok(Self {
            client: RwLock::new(client),
            graph_name: graph_name.to_string(),
        })
    }

    /// Create the AGE graph if it doesn't exist.
    ///
    /// This should be called from the `setup` command, not during normal connection.
    pub fn create_graph_if_not_exists(&self) -> Result<bool, Box<dyn Error>> {
        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        if Self::graph_exists(&mut client, &self.graph_name)? {
            Ok(false) // Graph already exists
        } else {
            client.create_graph(&self.graph_name)
                .map_err(|e| format!("Failed to create graph '{}': {}", self.graph_name, e))?;
            Ok(true) // Graph was created
        }
    }

    /// Verify that the AGE extension is installed and loaded.
    fn verify_age_extension(client: &mut Client) -> Result<(), Box<dyn Error>> {
        // Try to query ag_graph to verify AGE is available
        let result = client.simple_query("SELECT * FROM ag_catalog.ag_graph LIMIT 1");

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("ag_catalog") || err_str.contains("does not exist") {
                    Err("Apache AGE extension is not installed or not loaded. \
                         Please run: CREATE EXTENSION IF NOT EXISTS age; LOAD 'age';".into())
                } else {
                    Err(format!("Failed to verify AGE extension: {}", e).into())
                }
            }
        }
    }

    /// Check if a graph with the given name exists.
    fn graph_exists(client: &mut Client, graph_name: &str) -> Result<bool, Box<dyn Error>> {
        let query = format!(
            "SELECT 1 FROM ag_catalog.ag_graph WHERE name = '{}'",
            graph_name
        );
        let rows = client.simple_query(&query)?;
        Ok(!rows.is_empty())
    }

    /// Get the graph name for this backend.
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }
}

// Re-export conversion utilities
pub use conversion::*;
pub use schema::*;

/// Implement DatabaseBackend trait for PostgresAgeBackend.
///
/// This is a placeholder implementation for ticket #55a.
/// Full implementations of query methods will be completed in #55b and #55c.
impl DatabaseBackend for PostgresAgeBackend {
    fn execute_query(
        &self,
        _script: &str,
        _params: &Params,
    ) -> Result<QueryResult<DataValue>, Box<dyn Error>> {
        Err("PostgreSQL query execution not yet implemented - see ticket #55c".into())
    }

    fn backend_name(&self) -> &'static str {
        "PostgresAge"
    }

    fn relation_exists(&self, _name: &str) -> Result<bool, Box<dyn Error>> {
        Err("PostgreSQL relation existence check not yet implemented - see ticket #55b".into())
    }

    fn try_create_relation(&self, _schema: &str) -> Result<bool, Box<dyn Error>> {
        Err("PostgreSQL relation creation not yet implemented - see ticket #55b1".into())
    }

    fn insert_rows(
        &self,
        _relation: &SchemaRelation,
        _rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        Err("PostgreSQL row insertion not yet implemented - see ticket #55b".into())
    }

    fn delete_by_project(
        &self,
        _relation: &SchemaRelation,
        _project: &str,
    ) -> Result<usize, Box<dyn Error>> {
        Err("PostgreSQL row deletion not yet implemented - see ticket #55b".into())
    }

    fn as_db_instance(&self) -> &DbInstance {
        panic!("PostgreSQL backend does not use DbInstance - see ticket #55c")
    }
}
