//! Database schema creation and management.
//!
//! This module provides shared schema utilities used by both the import
//! and setup commands. It handles both CozoDB (single-pass creation) and
//! SurrealDB (two-phase creation) backends.

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
/// Handles backend-specific creation logic:
/// - **CozoDB**: Single-pass creation of all relations
/// - **SurrealDB**: Two-phase creation (nodes first, then relationships)
///
/// Returns a list of all relations with their creation status.
/// If a relation already exists, returns Ok with created=false for that relation.
pub fn create_schema(
    db: &dyn crate::backend::Database,
) -> Result<Vec<SchemaCreationResult>, Box<dyn Error>> {
    #[cfg(feature = "backend-cozo")]
    {
        create_schema_cozo(db)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        create_schema_surrealdb(db)
    }

    #[cfg(not(any(feature = "backend-cozo", feature = "backend-surrealdb")))]
    {
        compile_error!("Must enable either backend-cozo or backend-surrealdb")
    }
}

/// CozoDB schema creation: single-pass creation of all relations
#[cfg(feature = "backend-cozo")]
fn create_schema_cozo(
    db: &dyn crate::backend::Database,
) -> Result<Vec<SchemaCreationResult>, Box<dyn Error>> {
    use crate::backend::cozo_schema;

    let mut result = Vec::new();

    // CozoDB: Single pass, all relations at once
    let relation_names = [
        "modules",
        "functions",
        "calls",
        "struct_fields",
        "function_locations",
        "specs",
        "types",
    ];

    for name in relation_names {
        let script = cozo_schema::schema_for_relation(name)
            .ok_or_else(|| format!("Missing schema for relation: {}", name))?;
        let created = try_create_relation(db, script)?;
        result.push(SchemaCreationResult {
            relation: name.to_string(),
            created,
        });
    }

    Ok(result)
}

/// SurrealDB schema creation: two-phase creation (nodes first, then relationships)
#[cfg(feature = "backend-surrealdb")]
fn create_schema_surrealdb(
    db: &dyn crate::backend::Database,
) -> Result<Vec<SchemaCreationResult>, Box<dyn Error>> {
    use crate::backend::surrealdb_schema;

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
/// Returns the appropriate list for the active backend:
/// - **CozoDB**: 7 relations (modules, functions, calls, struct_fields, function_locations, specs, types)
/// - **SurrealDB**: 9 tables (5 nodes + 4 relationships, in creation order)
pub fn relation_names() -> Vec<&'static str> {
    #[cfg(feature = "backend-cozo")]
    {
        vec![
            "modules",
            "functions",
            "calls",
            "struct_fields",
            "function_locations",
            "specs",
            "types",
        ]
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        use crate::backend::surrealdb_schema;
        let mut names = Vec::new();
        names.extend_from_slice(surrealdb_schema::node_tables());
        names.extend_from_slice(surrealdb_schema::relationship_tables());
        names
    }

    #[cfg(not(any(feature = "backend-cozo", feature = "backend-surrealdb")))]
    {
        compile_error!("Must enable either backend-cozo or backend-surrealdb")
    }
}

/// Get schema script for a specific relation by name.
///
/// Routes to the appropriate backend schema module:
/// - **CozoDB**: Uses `cozo_schema::schema_for_relation`
/// - **SurrealDB**: Uses `surrealdb_schema::schema_for_table`
#[allow(dead_code)]
pub fn schema_for_relation(name: &str) -> Option<&'static str> {
    #[cfg(feature = "backend-cozo")]
    {
        use crate::backend::cozo_schema;
        cozo_schema::schema_for_relation(name)
    }

    #[cfg(feature = "backend-surrealdb")]
    {
        use crate::backend::surrealdb_schema;
        surrealdb_schema::schema_for_table(name)
    }

    #[cfg(not(any(feature = "backend-cozo", feature = "backend-surrealdb")))]
    {
        compile_error!("Must enable either backend-cozo or backend-surrealdb")
    }
}

#[cfg(all(test, feature = "backend-cozo"))]
mod cozo_tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_create_schema_creates_seven_relations() {
        let db = open_mem_db().expect("Failed to create in-memory DB");
        let result = create_schema(&*db).expect("Schema creation should succeed");

        // CozoDB should create 7 relations
        assert_eq!(result.len(), 7, "Should create exactly 7 relations");

        // All should be newly created
        assert!(
            result.iter().all(|r| r.created),
            "All relations should be newly created"
        );
    }

    #[test]
    fn test_create_schema_has_correct_relation_names() {
        let db = open_mem_db().expect("Failed to create in-memory DB");
        let result = create_schema(&*db).expect("Schema creation should succeed");

        let relation_names: Vec<_> = result.iter().map(|r| r.relation.as_str()).collect();

        // Verify all expected relation names are present
        assert!(
            relation_names.contains(&"modules"),
            "Should include modules relation"
        );
        assert!(
            relation_names.contains(&"functions"),
            "Should include functions relation"
        );
        assert!(
            relation_names.contains(&"calls"),
            "Should include calls relation"
        );
        assert!(
            relation_names.contains(&"struct_fields"),
            "Should include struct_fields relation"
        );
        assert!(
            relation_names.contains(&"function_locations"),
            "Should include function_locations relation"
        );
        assert!(
            relation_names.contains(&"specs"),
            "Should include specs relation"
        );
        assert!(
            relation_names.contains(&"types"),
            "Should include types relation"
        );
    }

    #[test]
    fn test_create_schema_is_idempotent() {
        let db = open_mem_db().expect("Failed to create in-memory DB");

        // First call should create all relations
        let result1 = create_schema(&*db).expect("First schema creation should succeed");
        assert_eq!(result1.len(), 7);
        assert!(
            result1.iter().all(|r| r.created),
            "First call should create all relations"
        );

        // Second call should find existing relations
        let result2 = create_schema(&*db).expect("Second schema creation should succeed");
        assert_eq!(result2.len(), 7);
        assert!(
            result2.iter().all(|r| !r.created),
            "Second call should find all relations already exist"
        );
    }

    #[test]
    fn test_relation_names_returns_correct_list() {
        let names = relation_names();

        assert_eq!(names.len(), 7, "Should return 7 relation names");
        assert!(names.contains(&"modules"));
        assert!(names.contains(&"functions"));
        assert!(names.contains(&"calls"));
        assert!(names.contains(&"struct_fields"));
        assert!(names.contains(&"function_locations"));
        assert!(names.contains(&"specs"));
        assert!(names.contains(&"types"));
    }

    #[test]
    fn test_schema_for_relation_returns_valid_ddl() {
        // Test that each relation has a valid schema definition
        let relations = [
            "modules",
            "functions",
            "calls",
            "struct_fields",
            "function_locations",
            "specs",
            "types",
        ];

        for relation in relations {
            let schema = schema_for_relation(relation);
            assert!(
                schema.is_some(),
                "Schema for {} should exist",
                relation
            );
            assert!(
                !schema.unwrap().is_empty(),
                "Schema for {} should not be empty",
                relation
            );
            assert!(
                schema.unwrap().contains(":create"),
                "Schema for {} should contain :create directive",
                relation
            );
        }
    }

    #[test]
    fn test_schema_for_relation_returns_none_for_invalid_name() {
        let schema = schema_for_relation("nonexistent_relation");
        assert!(
            schema.is_none(),
            "Should return None for invalid relation name"
        );
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_create_schema_creates_nine_tables() {
        let db = open_mem_db().expect("Failed to create in-memory DB");
        let result = create_schema(&*db).expect("Schema creation should succeed");

        // SurrealDB should create 9 tables (5 nodes + 4 relationships)
        assert_eq!(result.len(), 9, "Should create exactly 9 tables");

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
        // Node tables
        assert!(
            table_names.contains(&"module"),
            "Should include module node table"
        );
        assert!(
            table_names.contains(&"function"),
            "Should include function node table"
        );
        assert!(
            table_names.contains(&"clause"),
            "Should include clause node table"
        );
        assert!(
            table_names.contains(&"type"),
            "Should include type node table"
        );
        assert!(
            table_names.contains(&"field"),
            "Should include field node table"
        );

        // Relationship tables
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

        // Node tables should come first (5 tables)
        let node_tables = &table_names[0..5];
        assert!(
            node_tables.contains(&"module"),
            "Node tables should include module"
        );
        assert!(
            node_tables.contains(&"function"),
            "Node tables should include function"
        );
        assert!(
            node_tables.contains(&"clause"),
            "Node tables should include clause"
        );
        assert!(
            node_tables.contains(&"type"),
            "Node tables should include type"
        );
        assert!(
            node_tables.contains(&"field"),
            "Node tables should include field"
        );

        // Relationship tables should come after (4 tables)
        let rel_tables = &table_names[5..9];
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
        assert_eq!(result1.len(), 9);
        assert!(
            result1.iter().all(|r| r.created),
            "First call should create all tables"
        );

        // Second call should find existing tables
        let result2 = create_schema(&*db).expect("Second schema creation should succeed");
        assert_eq!(result2.len(), 9);
        assert!(
            result2.iter().all(|r| !r.created),
            "Second call should find all tables already exist"
        );
    }

    #[test]
    fn test_relation_names_returns_correct_list() {
        let names = relation_names();

        assert_eq!(names.len(), 9, "Should return 9 table names");

        // Node tables
        assert!(names.contains(&"module"));
        assert!(names.contains(&"function"));
        assert!(names.contains(&"clause"));
        assert!(names.contains(&"type"));
        assert!(names.contains(&"field"));

        // Relationship tables
        assert!(names.contains(&"defines"));
        assert!(names.contains(&"has_clause"));
        assert!(names.contains(&"calls"));
        assert!(names.contains(&"has_field"));
    }

    #[test]
    fn test_relation_names_preserves_creation_order() {
        let names = relation_names();

        // First 5 should be node tables
        let node_tables = &names[0..5];
        assert!(node_tables.contains(&"module"));
        assert!(node_tables.contains(&"function"));
        assert!(node_tables.contains(&"clause"));
        assert!(node_tables.contains(&"type"));
        assert!(node_tables.contains(&"field"));

        // Last 4 should be relationship tables
        let rel_tables = &names[5..9];
        assert!(rel_tables.contains(&"defines"));
        assert!(rel_tables.contains(&"has_clause"));
        assert!(rel_tables.contains(&"calls"));
        assert!(rel_tables.contains(&"has_field"));
    }

    #[test]
    fn test_schema_for_table_returns_valid_ddl() {
        // Test that each table has a valid schema definition
        let tables = [
            "module",
            "function",
            "clause",
            "type",
            "field",
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
        use crate::backend::surrealdb_schema;

        let node_tables = surrealdb_schema::node_tables();
        let rel_tables = surrealdb_schema::relationship_tables();

        // Verify we have the expected counts
        assert_eq!(node_tables.len(), 5, "Should have 5 node tables");
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
