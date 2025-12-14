//! All database relation definitions.
//!
//! This module defines the 7 relations that form the complete database schema.
//! Each definition matches the current schema in src/queries/schema.rs exactly.

use super::definition::{DataType, SchemaField, SchemaRelation, SchemaRelationship};

/// Modules relation: project modules/namespaces
///
/// Key fields: project, name
/// Value fields: file, source
pub const MODULES: SchemaRelation = SchemaRelation {
    name: "modules",
    key_fields: &[
        SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "name",
            data_type: DataType::String,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "file",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "source",
            data_type: DataType::String,
            default: Some("unknown"),
        },
    ],
    relationships: &[],
};

/// Functions relation: function definitions
///
/// Key fields: project, module, name, arity
/// Value fields: return_type, args, source
pub const FUNCTIONS: SchemaRelation = SchemaRelation {
    name: "functions",
    key_fields: &[
        SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "module",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "name",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "arity",
            data_type: DataType::Int,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "return_type",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "args",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "source",
            data_type: DataType::String,
            default: Some("unknown"),
        },
    ],
    relationships: &[SchemaRelationship {
        name: "located_in",
        target: "modules",
        edge_type: "LOCATED_IN",
    }],
};

/// Calls relation: function call edges
///
/// Key fields: project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column
/// Value fields: call_type, caller_kind, callee_args
pub const CALLS: SchemaRelation = SchemaRelation {
    name: "calls",
    key_fields: &[
        SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "caller_module",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "caller_function",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "callee_module",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "callee_function",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "callee_arity",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "file",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "line",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "column",
            data_type: DataType::Int,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "call_type",
            data_type: DataType::String,
            default: Some("remote"),
        },
        SchemaField {
            name: "caller_kind",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "callee_args",
            data_type: DataType::String,
            default: Some(""),
        },
    ],
    relationships: &[SchemaRelationship {
        name: "calls_edge",
        target: "functions",
        edge_type: "CALLS",
    }],
};

/// Struct fields relation: struct/record field definitions
///
/// Key fields: project, module, field
/// Value fields: default_value, required, inferred_type
pub const STRUCT_FIELDS: SchemaRelation = SchemaRelation {
    name: "struct_fields",
    key_fields: &[
        SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "module",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "field",
            data_type: DataType::String,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "default_value",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "required",
            data_type: DataType::Bool,
            default: None,
        },
        SchemaField {
            name: "inferred_type",
            data_type: DataType::String,
            default: None,
        },
    ],
    relationships: &[],
};

/// Function locations relation: detailed function location metadata
///
/// Key fields: project, module, name, arity, line
/// Value fields: file, source_file_absolute, column, kind, start_line, end_line, pattern, guard, source_sha, ast_sha, complexity, max_nesting_depth, generated_by, macro_source
pub const FUNCTION_LOCATIONS: SchemaRelation = SchemaRelation {
    name: "function_locations",
    key_fields: &[
        SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "module",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "name",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "arity",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "line",
            data_type: DataType::Int,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "file",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "source_file_absolute",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "column",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "kind",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "start_line",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "end_line",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "pattern",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "guard",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "source_sha",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "ast_sha",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "complexity",
            data_type: DataType::Int,
            default: Some("1"),
        },
        SchemaField {
            name: "max_nesting_depth",
            data_type: DataType::Int,
            default: Some("0"),
        },
        SchemaField {
            name: "generated_by",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "macro_source",
            data_type: DataType::String,
            default: Some(""),
        },
    ],
    relationships: &[],
};

/// Specs relation: function specification/type annotations
///
/// Key fields: project, module, name, arity
/// Value fields: kind, line, inputs_string, return_string, full
pub const SPECS: SchemaRelation = SchemaRelation {
    name: "specs",
    key_fields: &[
        SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "module",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "name",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "arity",
            data_type: DataType::Int,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "kind",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "line",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "inputs_string",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "return_string",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "full",
            data_type: DataType::String,
            default: Some(""),
        },
    ],
    relationships: &[],
};

/// Types relation: type definitions
///
/// Key fields: project, module, name
/// Value fields: kind, params, line, definition
pub const TYPES: SchemaRelation = SchemaRelation {
    name: "types",
    key_fields: &[
        SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "module",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "name",
            data_type: DataType::String,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "kind",
            data_type: DataType::String,
            default: None,
        },
        SchemaField {
            name: "params",
            data_type: DataType::String,
            default: Some(""),
        },
        SchemaField {
            name: "line",
            data_type: DataType::Int,
            default: None,
        },
        SchemaField {
            name: "definition",
            data_type: DataType::String,
            default: Some(""),
        },
    ],
    relationships: &[],
};

/// Schema migrations relation: tracks which schema versions have been applied
///
/// Key fields: version
/// Value fields: description
pub const SCHEMA_MIGRATIONS: SchemaRelation = SchemaRelation {
    name: "schema_versions",
    key_fields: &[
        SchemaField {
            name: "version",
            data_type: DataType::Int,
            default: None,
        },
    ],
    value_fields: &[
        SchemaField {
            name: "description",
            data_type: DataType::String,
            default: Some(""),
        },
    ],
    relationships: &[],
};

/// All relations for easy iteration.
///
/// Contains references to all 7 database relations.
pub const ALL_RELATIONS: &[&SchemaRelation] = &[
    &MODULES,
    &FUNCTIONS,
    &CALLS,
    &STRUCT_FIELDS,
    &FUNCTION_LOCATIONS,
    &SPECS,
    &TYPES,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_relations_defined() {
        assert_eq!(ALL_RELATIONS.len(), 7);
    }

    #[test]
    fn test_modules_relation() {
        let rel = &MODULES;
        assert_eq!(rel.name, "modules");
        assert_eq!(rel.key_fields.len(), 2);
        assert_eq!(rel.value_fields.len(), 2);
        assert_eq!(rel.field_count(), 4);

        // Check key fields
        assert_eq!(rel.key_fields[0].name, "project");
        assert_eq!(rel.key_fields[1].name, "name");

        // Check value fields
        assert_eq!(rel.value_fields[0].name, "file");
        assert_eq!(rel.value_fields[0].default, Some(""));
        assert_eq!(rel.value_fields[1].name, "source");
        assert_eq!(rel.value_fields[1].default, Some("unknown"));
    }

    #[test]
    fn test_functions_relation() {
        let rel = &FUNCTIONS;
        assert_eq!(rel.name, "functions");
        assert_eq!(rel.key_fields.len(), 4);
        assert_eq!(rel.value_fields.len(), 3);
        assert_eq!(rel.field_count(), 7);

        // Check key fields
        assert_eq!(rel.key_fields[0].name, "project");
        assert_eq!(rel.key_fields[1].name, "module");
        assert_eq!(rel.key_fields[2].name, "name");
        assert_eq!(rel.key_fields[3].name, "arity");

        // Check value fields
        assert_eq!(rel.value_fields[0].name, "return_type");
        assert_eq!(rel.value_fields[1].name, "args");
        assert_eq!(rel.value_fields[2].name, "source");

        // Check relationships
        assert_eq!(rel.relationships.len(), 1);
        assert_eq!(rel.relationships[0].name, "located_in");
        assert_eq!(rel.relationships[0].target, "modules");
    }

    #[test]
    fn test_calls_relation() {
        let rel = &CALLS;
        assert_eq!(rel.name, "calls");
        assert_eq!(rel.key_fields.len(), 9);
        assert_eq!(rel.value_fields.len(), 3);
        assert_eq!(rel.field_count(), 12);

        // Check key fields
        assert_eq!(rel.key_fields[0].name, "project");
        assert_eq!(rel.key_fields[1].name, "caller_module");
        assert_eq!(rel.key_fields[2].name, "caller_function");
        assert_eq!(rel.key_fields[3].name, "callee_module");
        assert_eq!(rel.key_fields[4].name, "callee_function");
        assert_eq!(rel.key_fields[5].name, "callee_arity");
        assert_eq!(rel.key_fields[6].name, "file");
        assert_eq!(rel.key_fields[7].name, "line");
        assert_eq!(rel.key_fields[8].name, "column");

        // Check value fields
        assert_eq!(rel.value_fields[0].name, "call_type");
        assert_eq!(rel.value_fields[0].default, Some("remote"));
        assert_eq!(rel.value_fields[1].name, "caller_kind");
        assert_eq!(rel.value_fields[2].name, "callee_args");

        // Check relationships
        assert_eq!(rel.relationships.len(), 1);
        assert_eq!(rel.relationships[0].name, "calls_edge");
        assert_eq!(rel.relationships[0].target, "functions");
    }

    #[test]
    fn test_struct_fields_relation() {
        let rel = &STRUCT_FIELDS;
        assert_eq!(rel.name, "struct_fields");
        assert_eq!(rel.key_fields.len(), 3);
        assert_eq!(rel.value_fields.len(), 3);
        assert_eq!(rel.field_count(), 6);

        // Check key fields
        assert_eq!(rel.key_fields[0].name, "project");
        assert_eq!(rel.key_fields[1].name, "module");
        assert_eq!(rel.key_fields[2].name, "field");

        // Check value fields
        assert_eq!(rel.value_fields[0].name, "default_value");
        assert_eq!(rel.value_fields[1].name, "required");
        assert_eq!(rel.value_fields[2].name, "inferred_type");
    }

    #[test]
    fn test_function_locations_relation() {
        let rel = &FUNCTION_LOCATIONS;
        assert_eq!(rel.name, "function_locations");
        assert_eq!(rel.key_fields.len(), 5);
        assert_eq!(rel.value_fields.len(), 14);
        assert_eq!(rel.field_count(), 19);

        // Check key fields
        assert_eq!(rel.key_fields[0].name, "project");
        assert_eq!(rel.key_fields[1].name, "module");
        assert_eq!(rel.key_fields[2].name, "name");
        assert_eq!(rel.key_fields[3].name, "arity");
        assert_eq!(rel.key_fields[4].name, "line");

        // Check value fields (spot check)
        assert_eq!(rel.value_fields[0].name, "file");
        assert_eq!(rel.value_fields[1].name, "source_file_absolute");
        assert_eq!(rel.value_fields[1].default, Some(""));
        assert_eq!(rel.value_fields[10].name, "complexity");
        assert_eq!(rel.value_fields[10].default, Some("1"));
        assert_eq!(rel.value_fields[11].name, "max_nesting_depth");
        assert_eq!(rel.value_fields[11].default, Some("0"));
    }

    #[test]
    fn test_specs_relation() {
        let rel = &SPECS;
        assert_eq!(rel.name, "specs");
        assert_eq!(rel.key_fields.len(), 4);
        assert_eq!(rel.value_fields.len(), 5);
        assert_eq!(rel.field_count(), 9);

        // Check key fields
        assert_eq!(rel.key_fields[0].name, "project");
        assert_eq!(rel.key_fields[1].name, "module");
        assert_eq!(rel.key_fields[2].name, "name");
        assert_eq!(rel.key_fields[3].name, "arity");

        // Check value fields
        assert_eq!(rel.value_fields[0].name, "kind");
        assert_eq!(rel.value_fields[1].name, "line");
        assert_eq!(rel.value_fields[2].name, "inputs_string");
    }

    #[test]
    fn test_types_relation() {
        let rel = &TYPES;
        assert_eq!(rel.name, "types");
        assert_eq!(rel.key_fields.len(), 3);
        assert_eq!(rel.value_fields.len(), 4);
        assert_eq!(rel.field_count(), 7);

        // Check key fields
        assert_eq!(rel.key_fields[0].name, "project");
        assert_eq!(rel.key_fields[1].name, "module");
        assert_eq!(rel.key_fields[2].name, "name");

        // Check value fields
        assert_eq!(rel.value_fields[0].name, "kind");
        assert_eq!(rel.value_fields[1].name, "params");
        assert_eq!(rel.value_fields[2].name, "line");
        assert_eq!(rel.value_fields[3].name, "definition");
    }

    #[test]
    fn test_schema_field_types() {
        // Verify all fields have valid data types
        for relation in ALL_RELATIONS {
            for field in relation.all_fields() {
                // Should be one of String, Int, Float, Bool
                let _ = field.data_type.cozo_type();
                let _ = field.data_type.age_type();
            }
        }
    }

    #[test]
    fn test_all_relations_have_names() {
        for relation in ALL_RELATIONS {
            assert!(!relation.name.is_empty());
            assert!(relation.field_count() > 0);
        }
    }

    #[test]
    fn test_all_relations_findable_by_name() {
        let names = vec!["modules", "functions", "calls", "struct_fields", "function_locations", "specs", "types"];
        for name in names {
            let found = ALL_RELATIONS.iter().any(|r| r.name == name);
            assert!(found, "Relation {} not found in ALL_RELATIONS", name);
        }
    }

    #[test]
    fn test_key_fields_not_empty() {
        // All relations must have at least one key field
        for relation in ALL_RELATIONS {
            assert!(
                !relation.key_fields.is_empty(),
                "Relation {} has no key fields",
                relation.name
            );
        }
    }

    #[test]
    fn test_no_field_name_duplicates_within_relation() {
        for relation in ALL_RELATIONS {
            let mut names = Vec::new();
            for field in relation.all_fields() {
                assert!(
                    !names.contains(&field.name),
                    "Duplicate field name '{}' in relation '{}'",
                    field.name,
                    relation.name
                );
                names.push(field.name);
            }
        }
    }
}
