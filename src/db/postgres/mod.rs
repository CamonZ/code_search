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
    /// Ensure the AGE extension is installed in the database.
    ///
    /// This should be called from the `setup` command BEFORE calling `new()`.
    /// Uses a regular PostgreSQL connection to create the extension if it doesn't exist.
    ///
    /// # Arguments
    /// * `connection_string` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Ok(true)` if the extension was created
    /// * `Ok(false)` if the extension already existed
    ///
    /// # Errors
    /// Returns an error if the connection fails or the extension cannot be created.
    pub fn ensure_extension_installed(connection_string: &str) -> Result<bool, Box<dyn Error>> {
        use postgres::{Client, NoTls};

        let mut client = Client::connect(connection_string, NoTls)
            .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;

        // Check if extension exists
        let rows = client.query(
            "SELECT 1 FROM pg_extension WHERE extname = 'age'",
            &[],
        )?;

        if !rows.is_empty() {
            return Ok(false); // Already installed
        }

        // Create extension
        client.execute("CREATE EXTENSION IF NOT EXISTS age", &[])?;

        Ok(true) // Extension was created
    }

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
    /// If the AGE extension might not be installed, call `ensure_extension_installed()`
    /// first (typically from the setup command).
    ///
    /// # Example
    /// ```ignore
    /// // In setup command:
    /// PostgresAgeBackend::ensure_extension_installed(conn_str)?;
    ///
    /// // Then connect:
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
        let messages = client.simple_query(&query)?;
        let has_rows = messages.iter().any(|msg| {
            matches!(msg, postgres::SimpleQueryMessage::Row(_))
        });
        Ok(has_rows)
    }

    /// Get the graph name for this backend.
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }

    /// Initialize the schema (creates version table and labels).
    ///
    /// This should be called from the `setup` command after `create_graph_if_not_exists()`.
    ///
    /// # Returns
    /// The schema version after initialization
    ///
    /// # Errors
    /// Returns an error if schema initialization fails
    pub fn initialize_schema(&self) -> Result<i32, Box<dyn Error>> {
        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        schema::initialize_schema(&mut client, &self.graph_name)
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
        params: &Params,
    ) -> Result<QueryResult<DataValue>, Box<dyn Error>> {
        // Substitute parameters into the query string
        // AGE's parameter handling via rust-postgres doesn't work well,
        // so we inline values directly (same approach as insert_rows)
        let cypher_query = conversion::substitute_params(script, params)?;

        // Acquire write lock on the client
        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        // Parse the Cypher query to extract column names from RETURN clause
        // and build the appropriate SQL wrapper
        let (sql_query, column_count) = conversion::wrap_cypher_query(&self.graph_name, &cypher_query)?;

        // Execute the query using raw SQL
        let rows = client.query(&sql_query, &[])
            .map_err(|e| format!("Cypher query failed: {}. Query was: {}", e, cypher_query))?;

        // Convert PostgreSQL rows to QueryResult<DataValue>
        conversion::convert_postgres_rows_to_query_result(&rows, column_count)
    }

    fn backend_name(&self) -> &'static str {
        "PostgresAge"
    }

    fn relation_exists(&self, name: &str) -> Result<bool, Box<dyn Error>> {
        // Convert relation name (e.g., "modules") to vertex label (e.g., "Module")
        let label_name = crate::db::schema::compilers::AgeCompiler::relation_to_vertex_label(name);

        // In AGE, we check if a vertex label exists in the ag_catalog
        let query = format!(
            "SELECT 1 FROM ag_catalog.ag_label WHERE name = '{}' AND graph = \
             (SELECT graphid FROM ag_catalog.ag_graph WHERE name = '{}')",
            label_name, self.graph_name
        );

        let mut client = self.client.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        // simple_query returns SimpleQueryMessage which includes CommandComplete
        // We need to check for actual Row messages
        let messages = client.simple_query(&query)?;
        let has_rows = messages.iter().any(|msg| {
            matches!(msg, postgres::SimpleQueryMessage::Row(_))
        });
        Ok(has_rows)
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

        // simple_query returns SimpleQueryMessage which includes CommandComplete
        // We need to check for actual Row messages
        let messages = client.simple_query(&exists_query)?;
        let has_rows = messages.iter().any(|msg| {
            matches!(msg, postgres::SimpleQueryMessage::Row(_))
        });
        if has_rows {
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
            // Convert rows to Cypher literal format (not JSON)
            // Cypher uses: [{key: 'value'}] not [{"key": "value"}]
            let rows_cypher = conversion::rows_to_cypher_literal(relation, chunk)?;

            // Build Cypher query with inlined data (no parameters)
            // AGE driver doesn't support ToSql for agtype, so we inline the data
            let cypher = conversion::compile_batch_insert_with_data(relation, &rows_cypher);

            let query = format!(
                "SELECT * FROM cypher('{}', $$ {} $$) AS (result agtype)",
                self.graph_name, cypher
            );

            // Execute without parameters - data is inlined in the query
            client.execute(&query, &[])?;

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

        // Inline the project value with escaping rather than using parameters
        // This avoids agtype conversion issues with the AGE driver
        let vertex_label = crate::db::schema::compilers::AgeCompiler::relation_to_vertex_label(relation.name);
        let escaped_project = project.replace('\'', "''");
        let cypher = format!(
            "MATCH (n:{})\nWHERE n.project = '{}'\nDETACH DELETE n",
            vertex_label, escaped_project
        );

        let query = format!(
            "SELECT * FROM cypher('{}', $$ {} $$) AS (result agtype)",
            self.graph_name, cypher
        );

        // Execute without parameters - value is inlined
        let result = client.execute(&query, &[])?;

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
            // Convert rows to Cypher literal format (not JSON)
            let rows_cypher = conversion::rows_to_cypher_literal(relation, chunk)?;

            // Build Cypher MERGE query with inlined data (no parameters)
            // AGE driver doesn't support ToSql for agtype, so we inline the data
            let cypher = conversion::compile_upsert_with_data(relation, &rows_cypher);

            let query = format!(
                "SELECT * FROM cypher('{}', $$ {} $$) AS (result agtype)",
                self.graph_name, cypher
            );

            // Execute without parameters - data is inlined in the query
            client.execute(&query, &[])?;
            total_upserted += chunk.len();
        }

        Ok(total_upserted)
    }

    fn as_db_instance(&self) -> &DbInstance {
        panic!("PostgresAgeBackend does not have a Cozo DbInstance. \
                Use execute_query() instead.")
    }

    fn setup_backend(&self) -> Result<(), Box<dyn Error>> {
        // Create the AGE graph if it doesn't exist
        self.create_graph_if_not_exists()?;
        Ok(())
    }
}
