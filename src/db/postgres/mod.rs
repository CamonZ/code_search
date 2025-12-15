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
/// Provides full implementation of database backend operations for PostgreSQL with AGE.
impl DatabaseBackend for PostgresAgeBackend {
    fn execute_query(
        &self,
        script: &str,
        _params: &Params,
    ) -> Result<QueryResult<DataValue>, Box<dyn Error>> {
        // Acquire write lock on the client
        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        // Execute Cypher query without parameters for now
        // Full parameter support will be implemented in phase 2 if needed
        let _rows = client.query_cypher::<serde_json::Value>(
            &self.graph_name,
            script,
            None
        ).map_err(|e| format!("Cypher query failed: {}", e))?;

        // Placeholder: Convert postgres::Row results to QueryResult<DataValue>
        // Each row is a postgres::Row object with agtype column(s)
        // For now, return empty result - full implementation in #55c
        Ok(QueryResult {
            headers: vec![],
            rows: vec![],
        })
    }

    fn backend_name(&self) -> &'static str {
        "PostgresAge"
    }

    fn relation_exists(&self, name: &str) -> Result<bool, Box<dyn Error>> {
        // In AGE, we check if a vertex label exists in the ag_catalog
        let query = format!(
            "SELECT 1 FROM ag_catalog.ag_label WHERE name = '{}' AND graph = \
             (SELECT graphid FROM ag_catalog.ag_graph WHERE name = '{}')",
            name, self.graph_name
        );

        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        let rows = client.simple_query(&query)?;
        Ok(!rows.is_empty())
    }

    fn try_create_relation(&self, schema: &str) -> Result<bool, Box<dyn Error>> {
        // In AGE, we create vertex/edge labels
        // The schema string should be a CREATE statement or label name
        //
        // Note: AGE creates labels implicitly on first use, so this is mostly
        // for explicit label creation with constraints

        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        // Check if it already exists
        // Extract label name from schema (assumes format "CREATE ... label_name ..." or just "label_name")
        let label_name = schema.split_whitespace()
            .find(|s| !s.eq_ignore_ascii_case("CREATE") &&
                      !s.eq_ignore_ascii_case("VERTEX") &&
                      !s.eq_ignore_ascii_case("EDGE") &&
                      !s.eq_ignore_ascii_case("LABEL"))
            .unwrap_or(schema);

        let exists_query = format!(
            "SELECT 1 FROM ag_catalog.ag_label WHERE name = '{}' AND graph = \
             (SELECT graphid FROM ag_catalog.ag_graph WHERE name = '{}')",
            label_name, self.graph_name
        );

        let rows = client.simple_query(&exists_query)?;
        if !rows.is_empty() {
            return Ok(false); // Already exists
        }

        // Create the label
        // AGE doesn't have explicit CREATE LABEL - labels are created on first use
        // But we can create a dummy vertex/edge to ensure the label exists
        let create_query = format!(
            "SELECT * FROM cypher('{}', $$ CREATE (n:{}) RETURN n $$) AS (n agtype)",
            self.graph_name, label_name
        );

        // Delete the dummy node immediately
        let delete_query = format!(
            "SELECT * FROM cypher('{}', $$ MATCH (n:{}) DELETE n $$) AS (result agtype)",
            self.graph_name, label_name
        );

        client.simple_query(&create_query)?;
        client.simple_query(&delete_query)?;

        Ok(true)
    }

    fn insert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        if rows.is_empty() {
            return Ok(0);
        }

        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        let mut total_inserted = 0;

        // Process in chunks to avoid query size limits
        const CHUNK_SIZE: usize = 500;

        for chunk in rows.chunks(CHUNK_SIZE) {
            // Build UNWIND query for batch insert
            let cypher = crate::db::schema::compilers::AgeCompiler::compile_batch_insert(relation);

            // Convert rows to JSON array for UNWIND
            let rows_json = conversion::convert_rows_to_json(relation, chunk)?;

            // Execute the batch insert
            let query = format!(
                "SELECT * FROM cypher('{}', $$ {} $$, $1) AS (result agtype)",
                self.graph_name, cypher
            );

            // Use parameterized query with rows as JSON
            client.execute(&query, &[&rows_json])?;

            total_inserted += chunk.len();
        }

        Ok(total_inserted)
    }

    fn delete_by_project(
        &self,
        relation: &SchemaRelation,
        project: &str,
    ) -> Result<usize, Box<dyn Error>> {
        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        // Use the AGE compiler to generate the delete query
        let cypher = crate::db::schema::compilers::AgeCompiler::compile_delete_by_project(relation);

        let query = format!(
            "SELECT * FROM cypher('{}', $$ {} $$, $1) AS (result agtype)",
            self.graph_name, cypher
        );

        // Execute with project as parameter
        let result = client.execute(&query, &[&project])?;

        // PostgreSQL returns rows affected
        Ok(result as usize)
    }

    fn upsert_rows(
        &self,
        relation: &SchemaRelation,
        rows: Vec<Vec<DataValue>>,
    ) -> Result<usize, Box<dyn Error>> {
        if rows.is_empty() {
            return Ok(0);
        }

        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        let mut total_upserted = 0;

        const CHUNK_SIZE: usize = 500;

        for chunk in rows.chunks(CHUNK_SIZE) {
            // Use MERGE for upsert semantics
            let cypher = crate::db::schema::compilers::AgeCompiler::compile_upsert(relation);

            let rows_json = conversion::convert_rows_to_json(relation, chunk)?;

            let query = format!(
                "SELECT * FROM cypher('{}', $$ {} $$, $1) AS (result agtype)",
                self.graph_name, cypher
            );

            client.execute(&query, &[&rows_json])?;
            total_upserted += chunk.len();
        }

        Ok(total_upserted)
    }

    fn as_db_instance(&self) -> &DbInstance {
        panic!("PostgresAgeBackend does not have a Cozo DbInstance. \
                Use execute_query() instead.")
    }
}
