//! Core schema definition types.
//!
//! Provides a backend-agnostic type system for describing database schema.
//! These types form the foundation for both Cozo and AGE schema generation.

/// Represents a database data type.
///
/// Maps to both Cozo and AGE type systems via `cozo_type()` and `age_type()` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// String/text data
    String,
    /// Integer data
    Int,
    /// Floating point data
    Float,
    /// Boolean data
    Bool,
}

impl DataType {
    /// Returns the Cozo type name for this data type.
    pub fn cozo_type(&self) -> &'static str {
        match self {
            DataType::String => "String",
            DataType::Int => "Int",
            DataType::Float => "Float",
            DataType::Bool => "Bool",
        }
    }

    /// Returns the AGE type name for this data type.
    pub fn age_type(&self) -> &'static str {
        match self {
            DataType::String => "String",
            DataType::Int => "Integer",
            DataType::Float => "Float",
            DataType::Bool => "Boolean",
        }
    }
}

/// Represents a field in a schema relation.
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Field name (e.g., "project", "name", "line")
    pub name: &'static str,

    /// Field data type
    pub data_type: DataType,

    /// Default value (if any). None means no default.
    pub default: Option<&'static str>,
}

/// Represents a relationship between two relations.
///
/// Used for documentation and AGE schema generation.
#[derive(Debug, Clone)]
pub struct SchemaRelationship {
    /// Relationship name (e.g., "located_in", "calls_edge")
    pub name: &'static str,

    /// Target relation name
    pub target: &'static str,

    /// AGE edge type name (e.g., "LOCATED_IN", "CALLS")
    pub edge_type: &'static str,
}

/// Represents a complete database relation/table.
#[derive(Debug, Clone)]
pub struct SchemaRelation {
    /// Relation name (e.g., "modules", "functions")
    pub name: &'static str,

    /// Fields that form the key (must be unique)
    pub key_fields: &'static [SchemaField],

    /// Fields that are associated values
    pub value_fields: &'static [SchemaField],

    /// Relationships to other relations
    pub relationships: &'static [SchemaRelationship],
}

impl SchemaRelation {
    /// Returns all fields in this relation (key + value).
    pub fn all_fields(&self) -> impl Iterator<Item = &SchemaField> {
        self.key_fields.iter().chain(self.value_fields.iter())
    }

    /// Returns the total number of fields.
    pub fn field_count(&self) -> usize {
        self.key_fields.len() + self.value_fields.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datatype_cozo_types() {
        assert_eq!(DataType::String.cozo_type(), "String");
        assert_eq!(DataType::Int.cozo_type(), "Int");
        assert_eq!(DataType::Float.cozo_type(), "Float");
        assert_eq!(DataType::Bool.cozo_type(), "Bool");
    }

    #[test]
    fn test_datatype_age_types() {
        assert_eq!(DataType::String.age_type(), "String");
        assert_eq!(DataType::Int.age_type(), "Integer");
        assert_eq!(DataType::Float.age_type(), "Float");
        assert_eq!(DataType::Bool.age_type(), "Boolean");
    }

    #[test]
    fn test_schema_field_creation() {
        let field = SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        };
        assert_eq!(field.name, "project");
        assert_eq!(field.data_type, DataType::String);
        assert_eq!(field.default, None);
    }

    #[test]
    fn test_schema_field_with_default() {
        let field = SchemaField {
            name: "file",
            data_type: DataType::String,
            default: Some(""),
        };
        assert_eq!(field.default, Some(""));
    }

    #[test]
    fn test_schema_relationship_creation() {
        let rel = SchemaRelationship {
            name: "located_in",
            target: "modules",
            edge_type: "LOCATED_IN",
        };
        assert_eq!(rel.name, "located_in");
        assert_eq!(rel.target, "modules");
        assert_eq!(rel.edge_type, "LOCATED_IN");
    }

    #[test]
    fn test_schema_relation_all_fields() {
        // Use static arrays for 'static lifetime
        const KEY_FIELDS: &[SchemaField] = &[
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
        ];
        const VALUE_FIELDS: &[SchemaField] = &[
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
        ];
        let rel = SchemaRelation {
            name: "modules",
            key_fields: KEY_FIELDS,
            value_fields: VALUE_FIELDS,
            relationships: &[],
        };

        let all_fields: Vec<_> = rel.all_fields().collect();
        assert_eq!(all_fields.len(), 4);
        assert_eq!(all_fields[0].name, "project");
        assert_eq!(all_fields[1].name, "name");
        assert_eq!(all_fields[2].name, "file");
        assert_eq!(all_fields[3].name, "source");
    }

    #[test]
    fn test_schema_relation_field_count() {
        // Use static arrays for 'static lifetime
        const KEY_FIELDS: &[SchemaField] = &[SchemaField {
            name: "project",
            data_type: DataType::String,
            default: None,
        }];
        const VALUE_FIELDS: &[SchemaField] = &[SchemaField {
            name: "file",
            data_type: DataType::String,
            default: Some(""),
        }];
        let rel = SchemaRelation {
            name: "test",
            key_fields: KEY_FIELDS,
            value_fields: VALUE_FIELDS,
            relationships: &[],
        };

        assert_eq!(rel.field_count(), 2);
    }
}
