//! Database connection and query utilities for CozoDB.
//!
//! This module provides the database abstraction layer for the CLI tool:
//! - Connection management (SQLite-backed or in-memory for tests)
//! - Query execution with parameter binding
//! - Result row extraction with type-safe helpers
//!
//! # Architecture
//!
//! CozoDB is a Datalog database that stores call graph data in relations.
//! Queries are written in CozoScript (a Datalog variant) and return `NamedRows`
//! containing `DataValue` cells that must be extracted into Rust types.
//!
//! # Type Decisions
//!
//! **Why `i64` for arity/line numbers instead of `u32`?**
//! CozoDB returns all integers as `Num::Int(i64)`. Using `i64` throughout avoids
//! lossy conversions and potential panics. The semantic constraint (arity >= 0)
//! is enforced by the data source (Elixir AST), not runtime checks.
//!
//! **Why `CallRowLayout` with indices instead of serde deserialization?**
//! CozoDB returns rows as `Vec<DataValue>`, not JSON objects. The `CallRowLayout`
//! struct documents column positions for each query type, centralizing the
//! mapping in two factory methods rather than scattering magic numbers.
//!
//! **Why bare `String` for module/function names instead of newtypes?**
//! For a CLI tool, the complexity of newtype wrappers (`.0` access, `Into` impls,
//! derive macro limitations) outweighs the type safety benefit. Field names
//! (`module`, `name`) are sufficiently clear.

mod backend;
mod connection;
mod escape;
mod extraction;
mod query;
mod value;

// Re-export public items
// DatabaseBackend: Used by open_db() return type. External imports after Ticket #44.
#[allow(unused_imports)]
pub use backend::{DatabaseBackend, Params, QueryResult};
pub use connection::open_db;
#[cfg(test)]
pub use connection::open_mem_db_raw;

pub use escape::{escape_string, escape_string_single};

pub use extraction::{
    extract_bool, extract_call_from_row, extract_f64, extract_i64, extract_string,
    extract_string_or, CallRowLayout,
};

pub use query::{run_query, run_query_no_params, try_create_relation};

// DatabaseValue trait available for future backends
#[allow(unused_imports)]
pub use value::DatabaseValue;

use thiserror::Error;

/// Database error types
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Failed to open database '{path}': {message}")]
    OpenFailed { path: String, message: String },

    #[error("Query failed: {message}")]
    QueryFailed { message: String },

    #[error("Missing column '{name}' in query result")]
    MissingColumn { name: String },
}
