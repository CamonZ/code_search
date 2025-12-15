//! Schema initialization and versioning for PostgreSQL AGE backend.
//!
//! Handles:
//! - Schema version table creation and tracking
//! - Vertex label initialization
//! - Edge label initialization
//! - Schema migrations

use std::error::Error;
use apache_age::sync::Client;
use postgres::SimpleQueryMessage;

/// Current schema version for PostgreSQL AGE backend.
pub const SCHEMA_VERSION: i32 = 1;

/// Schema version tracking table name.
const VERSION_TABLE: &str = "_schema_version";

/// Initialize the PostgreSQL AGE schema.
///
/// This function:
/// 1. Creates the schema version tracking table if needed
/// 2. Checks the current schema version
/// 3. Applies any necessary migrations
/// 4. Creates all required vertex and edge labels
///
/// # Arguments
/// * `client` - PostgreSQL client connection
/// * `graph_name` - Name of the AGE graph
///
/// # Returns
/// The current schema version after initialization
pub fn initialize_schema(
    client: &mut Client,
    graph_name: &str,
) -> Result<i32, Box<dyn Error>> {
    // 1. Create version tracking table
    create_version_table(client)?;

    // 2. Get current version
    let current_version = get_schema_version(client)?;

    // 3. Apply migrations if needed
    if current_version < SCHEMA_VERSION {
        migrate_schema(client, graph_name, current_version, SCHEMA_VERSION)?;
        set_schema_version(client, SCHEMA_VERSION)?;
    }

    // 4. Ensure all labels exist
    ensure_labels_exist(client, graph_name)?;

    Ok(SCHEMA_VERSION)
}

/// Create the schema version tracking table if it doesn't exist.
fn create_version_table(client: &mut Client) -> Result<(), Box<dyn Error>> {
    let query = format!(
        "CREATE TABLE IF NOT EXISTS {} (
            id INTEGER PRIMARY KEY DEFAULT 1,
            version INTEGER NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            CONSTRAINT single_row CHECK (id = 1)
        )",
        VERSION_TABLE
    );

    client.simple_query(&query)?;

    // Insert initial version if table is empty
    let insert_query = format!(
        "INSERT INTO {} (id, version) VALUES (1, 0) ON CONFLICT (id) DO NOTHING",
        VERSION_TABLE
    );
    client.simple_query(&insert_query)?;

    Ok(())
}

/// Get the current schema version.
fn get_schema_version(client: &mut Client) -> Result<i32, Box<dyn Error>> {
    let query = format!("SELECT version FROM {} WHERE id = 1", VERSION_TABLE);
    let rows = client.simple_query(&query)?;

    // Parse version from result
    for row in rows {
        if let SimpleQueryMessage::Row(row) = row {
            if let Some(version_str) = row.get(0) {
                return version_str.parse::<i32>()
                    .map_err(|e| format!("Invalid schema version: {}", e).into());
            }
        }
    }

    Ok(0) // Default to version 0 if not found
}

/// Set the schema version.
fn set_schema_version(client: &mut Client, version: i32) -> Result<(), Box<dyn Error>> {
    let query = format!(
        "UPDATE {} SET version = {}, updated_at = CURRENT_TIMESTAMP WHERE id = 1",
        VERSION_TABLE, version
    );
    client.simple_query(&query)?;
    Ok(())
}

/// Apply schema migrations between versions.
fn migrate_schema(
    client: &mut Client,
    graph_name: &str,
    from_version: i32,
    to_version: i32,
) -> Result<(), Box<dyn Error>> {
    for version in (from_version + 1)..=to_version {
        match version {
            1 => migrate_v0_to_v1(client, graph_name)?,
            _ => return Err(format!("Unknown migration version: {}", version).into()),
        }
    }
    Ok(())
}

/// Migration from v0 to v1: Initial schema setup.
fn migrate_v0_to_v1(client: &mut Client, graph_name: &str) -> Result<(), Box<dyn Error>> {
    // Create all vertex labels
    let vertex_labels = [
        "Function",
        "Module",
        "Struct",
        "Behaviour",
    ];

    for label in vertex_labels {
        create_vertex_label(client, graph_name, label)?;
    }

    // Create all edge labels
    let edge_labels = [
        "calls",
        "defines",
        "imports",
        "implements",
        "accepts",
        "returns",
    ];

    for label in edge_labels {
        create_edge_label(client, graph_name, label)?;
    }

    Ok(())
}

/// Ensure all required labels exist in the graph.
fn ensure_labels_exist(client: &mut Client, graph_name: &str) -> Result<(), Box<dyn Error>> {
    // Vertex labels
    let vertex_labels = ["Function", "Module", "Struct", "Behaviour"];
    for label in vertex_labels {
        ensure_vertex_label(client, graph_name, label)?;
    }

    // Edge labels
    let edge_labels = ["calls", "defines", "imports", "implements", "accepts", "returns"];
    for label in edge_labels {
        ensure_edge_label(client, graph_name, label)?;
    }

    Ok(())
}

/// Create a vertex label if it doesn't exist.
fn create_vertex_label(
    client: &mut Client,
    graph_name: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    // Check if label exists
    if label_exists(client, graph_name, label)? {
        return Ok(());
    }

    // AGE creates labels implicitly when first vertex is created
    // We create and immediately delete a dummy vertex
    let create_query = format!(
        "SELECT * FROM cypher('{}', $$ CREATE (n:{} {{_init: true}}) RETURN n $$) AS (n agtype)",
        graph_name, label
    );

    let delete_query = format!(
        "SELECT * FROM cypher('{}', $$ MATCH (n:{} {{_init: true}}) DELETE n $$) AS (result agtype)",
        graph_name, label
    );

    client.simple_query(&create_query)?;
    client.simple_query(&delete_query)?;

    Ok(())
}

/// Create an edge label if it doesn't exist.
fn create_edge_label(
    client: &mut Client,
    graph_name: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    // Check if label exists
    if label_exists(client, graph_name, label)? {
        return Ok(());
    }

    // For edges, we need to create two temp vertices, an edge, then delete all
    let setup_query = format!(
        "SELECT * FROM cypher('{}', $$
            CREATE (a:_TempInit)-[r:{}]->(b:_TempInit)
            RETURN r
        $$) AS (r agtype)",
        graph_name, label
    );

    let cleanup_query = format!(
        "SELECT * FROM cypher('{}', $$
            MATCH (n:_TempInit) DETACH DELETE n
        $$) AS (result agtype)",
        graph_name
    );

    client.simple_query(&setup_query)?;
    client.simple_query(&cleanup_query)?;

    Ok(())
}

/// Ensure a vertex label exists (idempotent).
fn ensure_vertex_label(
    client: &mut Client,
    graph_name: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    if !label_exists(client, graph_name, label)? {
        create_vertex_label(client, graph_name, label)?;
    }
    Ok(())
}

/// Ensure an edge label exists (idempotent).
fn ensure_edge_label(
    client: &mut Client,
    graph_name: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    if !label_exists(client, graph_name, label)? {
        create_edge_label(client, graph_name, label)?;
    }
    Ok(())
}

/// Check if a label exists in the graph.
fn label_exists(
    client: &mut Client,
    graph_name: &str,
    label: &str,
) -> Result<bool, Box<dyn Error>> {
    let query = format!(
        "SELECT 1 FROM ag_catalog.ag_label
         WHERE name = '{}'
         AND graph = (SELECT graphid FROM ag_catalog.ag_graph WHERE name = '{}')",
        label, graph_name
    );

    let rows = client.simple_query(&query)?;

    // Check if any rows were returned
    for row in rows {
        if let SimpleQueryMessage::Row(_) = row {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Drop the schema (for testing purposes).
#[cfg(test)]
pub fn drop_schema(client: &mut Client, graph_name: &str) -> Result<(), Box<dyn Error>> {
    // Drop all data in graph
    let drop_query = format!(
        "SELECT * FROM cypher('{}', $$ MATCH (n) DETACH DELETE n $$) AS (result agtype)",
        graph_name
    );
    client.simple_query(&drop_query)?;

    // Reset version
    let reset_query = format!(
        "UPDATE {} SET version = 0, updated_at = CURRENT_TIMESTAMP WHERE id = 1",
        VERSION_TABLE
    );
    client.simple_query(&reset_query)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version_constant() {
        assert!(SCHEMA_VERSION >= 1);
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_version_table_constant() {
        assert_eq!(VERSION_TABLE, "_schema_version");
    }
}
