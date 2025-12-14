//! Database schema compilers.
//!
//! Generates backend-specific DDL from backend-agnostic schema definitions.
//! Each compiler converts `SchemaRelation` definitions into the target database's
//! native DDL syntax.

pub mod cozo;

pub use cozo::CozoCompiler;
