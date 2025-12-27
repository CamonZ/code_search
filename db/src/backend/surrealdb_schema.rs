//! SurrealDB graph schema module.
//!
//! Defines the complete graph schema for SurrealDB with 5 node tables and 4 relationship tables.
//! Uses `SCHEMAFULL` mode for strict schema enforcement and unique indexes on natural keys.

// Node Tables (5 entities)

/// Schema definition for the module node table.
///
/// Represents code modules with unique identification by name.
/// No project field - database is one per project.
pub const SCHEMA_MODULE: &str = r#"
DEFINE TABLE module SCHEMAFULL;
DEFINE FIELD name ON module TYPE string;
DEFINE FIELD file ON module TYPE string DEFAULT "";
DEFINE FIELD source ON module TYPE string DEFAULT "unknown";
DEFINE INDEX idx_module_name ON module FIELDS name UNIQUE;
"#;

/// Schema definition for the function node table.
///
/// Represents function identities with signature (module_name, name, arity).
/// Derived from function_locations - represents a unique function regardless of clause count.
pub const SCHEMA_FUNCTION: &str = r#"
DEFINE TABLE function SCHEMAFULL;
DEFINE FIELD module_name ON function TYPE string;
DEFINE FIELD name ON function TYPE string;
DEFINE FIELD arity ON function TYPE int;
DEFINE INDEX idx_function_natural_key ON function FIELDS module_name, name, arity UNIQUE;
DEFINE INDEX idx_function_module ON function FIELDS module_name;
DEFINE INDEX idx_function_name ON function FIELDS name;
"#;

/// Schema definition for the clause node table.
///
/// Represents individual function clauses (pattern-matched heads).
/// Renamed from CozoDB's `function_locations` for clearer semantics.
/// Unique key: (module_name, function_name, arity, line)
pub const SCHEMA_CLAUSE: &str = r#"
DEFINE TABLE clause SCHEMAFULL;
DEFINE FIELD module_name ON clause TYPE string;
DEFINE FIELD function_name ON clause TYPE string;
DEFINE FIELD arity ON clause TYPE int;
DEFINE FIELD line ON clause TYPE int;
DEFINE FIELD source_file ON clause TYPE string;
DEFINE FIELD source_file_absolute ON clause TYPE string DEFAULT "";
DEFINE FIELD kind ON clause TYPE string;
DEFINE FIELD start_line ON clause TYPE int;
DEFINE FIELD end_line ON clause TYPE int;
DEFINE FIELD pattern ON clause TYPE string DEFAULT "";
DEFINE FIELD guard ON clause TYPE option<string>;
DEFINE FIELD source_sha ON clause TYPE string DEFAULT "";
DEFINE FIELD ast_sha ON clause TYPE string DEFAULT "";
DEFINE FIELD complexity ON clause TYPE int DEFAULT 1;
DEFINE FIELD max_nesting_depth ON clause TYPE int DEFAULT 0;
DEFINE FIELD generated_by ON clause TYPE option<string>;
DEFINE FIELD macro_source ON clause TYPE option<string>;
DEFINE INDEX idx_clause_natural_key ON clause FIELDS module_name, function_name, arity, line UNIQUE;
DEFINE INDEX idx_clause_function ON clause FIELDS module_name, function_name, arity;
"#;

/// Schema definition for the spec node table.
///
/// Represents @spec and @callback definitions.
/// A spec belongs to a module and references a function (by name and arity).
/// Specs can have multiple clauses (for overloaded functions), each stored as a separate row.
/// Unique key: (module_name, function_name, arity, clause_index)
pub const SCHEMA_SPEC: &str = r#"
DEFINE TABLE spec SCHEMAFULL;
DEFINE FIELD module_name ON spec TYPE string;
DEFINE FIELD function_name ON spec TYPE string;
DEFINE FIELD arity ON spec TYPE int;
DEFINE FIELD kind ON spec TYPE string;
DEFINE FIELD line ON spec TYPE int;
DEFINE FIELD clause_index ON spec TYPE int DEFAULT 0;
DEFINE FIELD input_strings ON spec TYPE array<string> DEFAULT [];
DEFINE FIELD return_strings ON spec TYPE array<string> DEFAULT [];
DEFINE FIELD full ON spec TYPE string DEFAULT "";
DEFINE INDEX idx_spec_natural_key ON spec FIELDS module_name, function_name, arity, clause_index UNIQUE;
DEFINE INDEX idx_spec_module ON spec FIELDS module_name;
DEFINE INDEX idx_spec_function ON spec FIELDS module_name, function_name, arity;
"#;

/// Schema definition for the type node table.
///
/// Represents @type, @typep, and @opaque definitions within modules.
/// Unique key: (module_name, name)
pub const SCHEMA_TYPE: &str = r#"
DEFINE TABLE type SCHEMAFULL;
DEFINE FIELD module_name ON type TYPE string;
DEFINE FIELD name ON type TYPE string;
DEFINE FIELD kind ON type TYPE string;
DEFINE FIELD params ON type TYPE string DEFAULT "";
DEFINE FIELD line ON type TYPE int;
DEFINE FIELD definition ON type TYPE string DEFAULT "";
DEFINE INDEX idx_type_natural_key ON type FIELDS module_name, name UNIQUE;
DEFINE INDEX idx_type_module ON type FIELDS module_name;
DEFINE INDEX idx_type_name ON type FIELDS name;
"#;

/// Schema definition for the field node table.
///
/// Represents struct fields within a module.
/// A module can define at most one struct, and the struct name equals the module name.
/// Unique key: (module_name, name)
pub const SCHEMA_FIELD: &str = r#"
DEFINE TABLE field SCHEMAFULL;
DEFINE FIELD module_name ON field TYPE string;
DEFINE FIELD name ON field TYPE string;
DEFINE FIELD default_value ON field TYPE string;
DEFINE FIELD required ON field TYPE bool;
DEFINE INDEX idx_field_natural_key ON field FIELDS module_name, name UNIQUE;
DEFINE INDEX idx_field_module ON field FIELDS module_name;
DEFINE INDEX idx_field_name ON field FIELDS name;
"#;

// Relationship Tables (4 edges)

/// Schema definition for the defines relationship table.
///
/// Represents module containment: module -> function | type | spec
/// Graph edge enabling traversal of what entities a module defines.
pub const SCHEMA_DEFINES: &str = r#"
DEFINE TABLE defines SCHEMAFULL TYPE RELATION FROM module TO function | type | spec;
DEFINE INDEX idx_defines_in ON defines FIELDS in;
DEFINE INDEX idx_defines_out ON defines FIELDS out;
"#;

/// Schema definition for the has_clause relationship table.
///
/// Represents function clause membership: function -> clause
/// Graph edge linking functions to their individual clauses (pattern-matched heads).
pub const SCHEMA_HAS_CLAUSE: &str = r#"
DEFINE TABLE has_clause SCHEMAFULL TYPE RELATION FROM function TO clause;
DEFINE INDEX idx_has_clause_in ON has_clause FIELDS in;
DEFINE INDEX idx_has_clause_out ON has_clause FIELDS out;
"#;

/// Schema definition for the calls relationship table.
///
/// Represents the call graph: function -> function
/// Includes metadata about the call and reference to the specific clause where it occurs.
pub const SCHEMA_CALLS: &str = r#"
DEFINE TABLE calls SCHEMAFULL TYPE RELATION FROM function TO function;
DEFINE FIELD call_type ON calls TYPE string DEFAULT "remote";
DEFINE FIELD caller_kind ON calls TYPE string DEFAULT "";
DEFINE FIELD file ON calls TYPE string;
DEFINE FIELD line ON calls TYPE int;
DEFINE FIELD caller_clause_id ON calls TYPE option<record<clause>>;
DEFINE INDEX idx_calls_in ON calls FIELDS in;
DEFINE INDEX idx_calls_out ON calls FIELDS out;
DEFINE INDEX idx_calls_file ON calls FIELDS file;
DEFINE INDEX idx_calls_caller_clause ON calls FIELDS caller_clause_id;
"#;

/// Schema definition for the has_field relationship table.
///
/// Represents struct field membership: module -> field
/// Graph edge linking modules (that define structs) to their fields.
pub const SCHEMA_HAS_FIELD: &str = r#"
DEFINE TABLE has_field SCHEMAFULL TYPE RELATION FROM module TO field;
DEFINE INDEX idx_has_field_in ON has_field FIELDS in;
DEFINE INDEX idx_has_field_out ON has_field FIELDS out;
"#;

/// Retrieves the schema definition for a specific table by name.
///
/// Returns the complete schema DDL for the requested table, or None if not found.
///
/// # Arguments
/// * `name` - Table name ("module", "function", "clause", "spec", "type", "field", "defines", "has_clause", "calls", "has_field")
///
/// # Returns
/// * `Some(&str)` - The schema DDL for the table
/// * `None` - If the table name is not recognized
pub fn schema_for_table(name: &str) -> Option<&'static str> {
    match name {
        "module" => Some(SCHEMA_MODULE),
        "function" => Some(SCHEMA_FUNCTION),
        "clause" => Some(SCHEMA_CLAUSE),
        "spec" => Some(SCHEMA_SPEC),
        "type" => Some(SCHEMA_TYPE),
        "field" => Some(SCHEMA_FIELD),
        "defines" => Some(SCHEMA_DEFINES),
        "has_clause" => Some(SCHEMA_HAS_CLAUSE),
        "calls" => Some(SCHEMA_CALLS),
        "has_field" => Some(SCHEMA_HAS_FIELD),
        _ => None,
    }
}

/// Returns a slice of all node table names in dependency order.
///
/// Node tables have no external dependencies and should be created first.
pub fn node_tables() -> &'static [&'static str] {
    &["module", "function", "clause", "spec", "type", "field"]
}

/// Returns a slice of all relationship table names in dependency order.
///
/// Relationship tables depend on node tables and should be created after nodes.
pub fn relationship_tables() -> &'static [&'static str] {
    &["defines", "has_clause", "calls", "has_field"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tables_have_schemas() {
        let all_tables = [
            "module", "function", "clause", "spec", "type", "field",
            "defines", "has_clause", "calls", "has_field",
        ];

        for table in all_tables {
            assert!(
                schema_for_table(table).is_some(),
                "Missing schema for table: {}",
                table
            );
        }
    }

    #[test]
    fn test_schema_strings_are_valid_sql() {
        let all_tables = [
            "module", "function", "clause", "spec", "type", "field",
            "defines", "has_clause", "calls", "has_field",
        ];

        for table in all_tables {
            let schema = schema_for_table(table).expect(&format!("Missing schema for {}", table));
            assert!(!schema.is_empty(), "Empty schema for table: {}", table);
            assert!(
                schema.contains("DEFINE TABLE"),
                "Schema for {} doesn't contain DEFINE TABLE",
                table
            );
        }
    }

    #[test]
    fn test_all_schemas_use_schemafull() {
        let all_tables = [
            "module", "function", "clause", "spec", "type", "field",
            "defines", "has_clause", "calls", "has_field",
        ];

        for table in all_tables {
            let schema = schema_for_table(table).expect(&format!("Missing schema for {}", table));
            assert!(
                schema.contains("SCHEMAFULL"),
                "Schema for {} doesn't use SCHEMAFULL",
                table
            );
        }
    }

    #[test]
    fn test_node_and_relationship_tables_partition_all_tables() {
        let mut all_from_functions = std::collections::HashSet::new();

        for table in node_tables() {
            all_from_functions.insert(*table);
        }

        for table in relationship_tables() {
            all_from_functions.insert(*table);
        }

        assert_eq!(all_from_functions.len(), 10, "Should have 10 total tables");
    }

    #[test]
    fn test_natural_key_uniqueness_indexes() {
        // Verify that each table has appropriate unique indexes on natural keys

        // module: name
        let module_schema = schema_for_table("module").unwrap();
        assert!(module_schema.contains("UNIQUE"), "module should have UNIQUE index");

        // function: (module_name, name, arity)
        let function_schema = schema_for_table("function").unwrap();
        assert!(function_schema.contains("natural_key"), "function should have natural_key index");
        assert!(function_schema.contains("UNIQUE"), "function should have UNIQUE index");

        // type: (module_name, name)
        let type_schema = schema_for_table("type").unwrap();
        assert!(type_schema.contains("natural_key"), "type should have natural_key index");
        assert!(type_schema.contains("UNIQUE"), "type should have UNIQUE index");
    }
}
