//! Database schema compilers.
//!
//! Generates backend-specific DDL from backend-agnostic schema definitions.
//! Each compiler converts `SchemaRelation` definitions into the target database's
//! native DDL syntax.

pub mod cozo;
pub mod age;

pub use cozo::CozoCompiler;
pub use age::AgeCompiler;
