//! Database schema creation and management.
//!
//! This module provides shared schema utilities used by both the import
//! and setup commands.

use crate::backend::surrealdb_schema;
use crate::db::try_create_relation;
use std::error::Error;

/// Result of schema creation operation
#[derive(Debug, Clone)]
pub struct SchemaCreationResult {
    pub relation: String,
    pub created: bool,
}

/// Create all database schemas.
///
/// Two-phase creation: nodes first, then relationships.
/// Returns a list of all relations with their creation status.
/// If a relation already exists, returns Ok with created=false for that relation.
pub fn create_schema(
    db: &dyn crate::backend::Database,
) -> Result<Vec<SchemaCreationResult>, Box<dyn Error>> {
    let mut result = Vec::new();

    // Phase 1: Create node tables
    for name in surrealdb_schema::node_tables() {
        let script = surrealdb_schema::schema_for_table(name)
            .ok_or_else(|| format!("Missing schema for table: {}", name))?;
        let created = try_create_relation(db, script)?;
        result.push(SchemaCreationResult {
            relation: name.to_string(),
            created,
        });
    }

    // Phase 2: Create relationship tables (require nodes to exist)
    for name in surrealdb_schema::relationship_tables() {
        let script = surrealdb_schema::schema_for_table(name)
            .ok_or_else(|| format!("Missing schema for table: {}", name))?;
        let created = try_create_relation(db, script)?;
        result.push(SchemaCreationResult {
            relation: name.to_string(),
            created,
        });
    }

    Ok(result)
}

/// Get list of all relation names managed by this schema.
///
/// Returns 10 tables (6 nodes + 4 relationships, in creation order)
pub fn relation_names() -> Vec<&'static str> {
    let mut names = Vec::new();
    names.extend_from_slice(surrealdb_schema::node_tables());
    names.extend_from_slice(surrealdb_schema::relationship_tables());
    names
}

/// Get schema script for a specific relation by name.
#[allow(dead_code)]
pub fn schema_for_relation(name: &str) -> Option<&'static str> {
    surrealdb_schema::schema_for_table(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_create_schema_creates_ten_tables() {
        let db = open_mem_db().expect("Failed to create in-memory DB");
        let result = create_schema(&*db).expect("Schema creation should succeed");

        // SurrealDB should create 10 tables (6 nodes + 4 relationships)
        assert_eq!(result.len(), 10, "Should create exactly 10 tables");

        // All should be newly created
        assert!(
            result.iter().all(|r| r.created),
            "All tables should be newly created"
        );
    }

    #[test]
    fn test_create_schema_has_correct_table_names() {
        let db = open_mem_db().expect("Failed to create in-memory DB");
        let result = create_schema(&*db).expect("Schema creation should succeed");

        let table_names: Vec<_> = result.iter().map(|r| r.relation.as_str()).collect();

        // Verify all expected table names are present
        // Node tables (6)
        assert!(
            table_names.contains(&"modules"),
            "Should include modules node table"
        );
        assert!(
            table_names.contains(&"functions"),
            "Should include functions node table"
        );
        assert!(
            table_names.contains(&"clauses"),
            "Should include clauses node table"
        );
        assert!(
            table_names.contains(&"specs"),
            "Should include specs node table"
        );
        assert!(
            table_names.contains(&"types"),
            "Should include types node table"
        );
        assert!(
            table_names.contains(&"fields"),
            "Should include fields node table"
        );

        // Relationship tables (4)
        assert!(
            table_names.contains(&"defines"),
            "Should include defines relationship table"
        );
        assert!(
            table_names.contains(&"has_clause"),
            "Should include has_clause relationship table"
        );
        assert!(
            table_names.contains(&"calls"),
            "Should include calls relationship table"
        );
        assert!(
            table_names.contains(&"has_field"),
            "Should include has_field relationship table"
        );
    }

    #[test]
    fn test_create_schema_two_phase_order() {
        let db = open_mem_db().expect("Failed to create in-memory DB");
        let result = create_schema(&*db).expect("Schema creation should succeed");

        // Extract table names in creation order
        let table_names: Vec<_> = result.iter().map(|r| r.relation.as_str()).collect();

        // Node tables should come first (6 tables)
        let node_tables = &table_names[0..6];
        assert!(
            node_tables.contains(&"modules"),
            "Node tables should include modules"
        );
        assert!(
            node_tables.contains(&"functions"),
            "Node tables should include functions"
        );
        assert!(
            node_tables.contains(&"clauses"),
            "Node tables should include clauses"
        );
        assert!(
            node_tables.contains(&"specs"),
            "Node tables should include specs"
        );
        assert!(
            node_tables.contains(&"types"),
            "Node tables should include types"
        );
        assert!(
            node_tables.contains(&"fields"),
            "Node tables should include fields"
        );

        // Relationship tables should come after (4 tables)
        let rel_tables = &table_names[6..10];
        assert!(
            rel_tables.contains(&"defines"),
            "Relationship tables should include defines"
        );
        assert!(
            rel_tables.contains(&"has_clause"),
            "Relationship tables should include has_clause"
        );
        assert!(
            rel_tables.contains(&"calls"),
            "Relationship tables should include calls"
        );
        assert!(
            rel_tables.contains(&"has_field"),
            "Relationship tables should include has_field"
        );
    }

    #[test]
    fn test_create_schema_is_idempotent() {
        let db = open_mem_db().expect("Failed to create in-memory DB");

        // First call should create all tables
        let result1 = create_schema(&*db).expect("First schema creation should succeed");
        assert_eq!(result1.len(), 10);
        assert!(
            result1.iter().all(|r| r.created),
            "First call should create all tables"
        );

        // Second call should find existing tables
        let result2 = create_schema(&*db).expect("Second schema creation should succeed");
        assert_eq!(result2.len(), 10);
        assert!(
            result2.iter().all(|r| !r.created),
            "Second call should find all tables already exist"
        );
    }

    #[test]
    fn test_relation_names_returns_correct_list() {
        let names = relation_names();

        assert_eq!(names.len(), 10, "Should return 10 table names");

        // Node tables (6)
        assert!(names.contains(&"modules"));
        assert!(names.contains(&"functions"));
        assert!(names.contains(&"clauses"));
        assert!(names.contains(&"specs"));
        assert!(names.contains(&"types"));
        assert!(names.contains(&"fields"));

        // Relationship tables (4)
        assert!(names.contains(&"defines"));
        assert!(names.contains(&"has_clause"));
        assert!(names.contains(&"calls"));
        assert!(names.contains(&"has_field"));
    }

    #[test]
    fn test_relation_names_preserves_creation_order() {
        let names = relation_names();

        // First 6 should be node tables
        let node_tables = &names[0..6];
        assert!(node_tables.contains(&"modules"));
        assert!(node_tables.contains(&"functions"));
        assert!(node_tables.contains(&"clauses"));
        assert!(node_tables.contains(&"specs"));
        assert!(node_tables.contains(&"types"));
        assert!(node_tables.contains(&"fields"));

        // Last 4 should be relationship tables
        let rel_tables = &names[6..10];
        assert!(rel_tables.contains(&"defines"));
        assert!(rel_tables.contains(&"has_clause"));
        assert!(rel_tables.contains(&"calls"));
        assert!(rel_tables.contains(&"has_field"));
    }

    #[test]
    fn test_schema_for_table_returns_valid_ddl() {
        // Test that each table has a valid schema definition
        let tables = [
            "modules",
            "functions",
            "clauses",
            "specs",
            "types",
            "fields",
            "defines",
            "has_clause",
            "calls",
            "has_field",
        ];

        for table in tables {
            let schema = schema_for_relation(table);
            assert!(schema.is_some(), "Schema for {} should exist", table);
            assert!(
                !schema.unwrap().is_empty(),
                "Schema for {} should not be empty",
                table
            );
            assert!(
                schema.unwrap().contains("DEFINE TABLE"),
                "Schema for {} should contain DEFINE TABLE directive",
                table
            );
        }
    }

    #[test]
    fn test_schema_for_table_returns_none_for_invalid_name() {
        let schema = schema_for_relation("nonexistent_table");
        assert!(
            schema.is_none(),
            "Should return None for invalid table name"
        );
    }

    #[test]
    fn test_node_tables_defined_before_relationships() {
        let node_tables = surrealdb_schema::node_tables();
        let rel_tables = surrealdb_schema::relationship_tables();

        // Verify we have the expected counts
        assert_eq!(node_tables.len(), 6, "Should have 6 node tables");
        assert_eq!(rel_tables.len(), 4, "Should have 4 relationship tables");

        // Verify relationship tables reference node tables
        for rel_table in rel_tables {
            let schema = surrealdb_schema::schema_for_table(rel_table)
                .expect("Schema should exist for relationship table");

            // Relationship tables should have TYPE RELATION syntax
            assert!(
                schema.contains("TYPE RELATION"),
                "{} should be a RELATION type",
                rel_table
            );
        }
    }
}
