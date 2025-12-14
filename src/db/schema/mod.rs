//! Backend-agnostic database schema definitions.
//!
//! This module provides structured definitions for the database schema,
//! allowing both Cozo and AGE backends to be generated from a single source of truth.
//!
//! # Overview
//!
//! The schema system consists of three main components:
//!
//! 1. **Core Types** (`definition.rs`):
//!    - `DataType` - Enum representing database data types (String, Int, Float, Bool)
//!    - `SchemaField` - Represents a single field with name, type, and optional default
//!    - `SchemaRelation` - Represents a complete relation with key and value fields
//!    - `SchemaRelationship` - Documents relationships between relations
//!
//! 2. **Relation Definitions** (`relations.rs`):
//!    - `MODULES`, `FUNCTIONS`, `CALLS`, `STRUCT_FIELDS`, `FUNCTION_LOCATIONS`, `SPECS`, `TYPES`
//!    - `ALL_RELATIONS` - Slice for easy iteration over all 7 relations
//!
//! # Type Mapping
//!
//! Each `DataType` maps to both Cozo and AGE type systems:
//!
//! | Rust Type | Cozo Type | AGE Type |
//! |-----------|-----------|----------|
//! | String | String | String |
//! | Int | Int | Integer |
//! | Float | Float | Float |
//! | Bool | Bool | Boolean |

mod definition;
mod relations;

// Re-export public items
#[allow(unused_imports)]
pub use definition::{DataType, SchemaField, SchemaRelation, SchemaRelationship};
#[allow(unused_imports)]
pub use relations::{ALL_RELATIONS, CALLS, FUNCTION_LOCATIONS, FUNCTIONS, MODULES, SPECS, STRUCT_FIELDS, TYPES};
