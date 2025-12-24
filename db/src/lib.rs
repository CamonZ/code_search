//! Database layer for code search - database abstraction with backend support
//!
//! This crate provides a backend-agnostic database layer that supports multiple backends:
//! - **CozoDB** (Datalog-based, default) - Graph query language with SQLite storage
//! - **SurrealDB** (Multi-model database, future) - Document and graph database
//!
//! # Backend Selection
//!
//! Use Cargo features to select the database backend at compile time:
//!
//! ```toml
//! # Use CozoDB (default)
//! db = { path = "../db" }
//!
//! # Use SurrealDB
//! db = { path = "../db", default-features = false, features = ["backend-surrealdb"] }
//! ```
//!
//! # Architecture
//!
//! The database layer uses trait-based abstractions to support multiple backends:
//!
//! - [`Database`] trait - Connection and query execution
//! - [`QueryResult`] trait - Backend-agnostic result set
//! - [`Row`] trait - Individual row access
//! - [`Value`] trait - Type-safe value extraction
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use db::{open_db, Database, QueryParams};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open a database connection
//! let db = open_db(Path::new("my_database.db"))?;
//!
//! // Execute a query with parameters
//! let params = QueryParams::new()
//!     .with_str("project", "my_project");
//!
//! let result = db.execute_query(
//!     "?[module] := *modules{project: $project, module}",
//!     params
//! )?;
//!
//! // Access results
//! for row in result.rows() {
//!     if let Some(module) = row.get(0) {
//!         println!("Module: {:?}", module.as_str());
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod backend;
pub mod db;
pub mod types;
pub mod query_builders;
pub mod queries;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

#[cfg(any(test, feature = "test-utils"))]
pub mod fixtures;

// ============================================================================
// Backend Abstraction Exports
// ============================================================================

/// Core database trait for backend-agnostic operations
pub use backend::Database;

/// Query result trait for accessing query results
pub use backend::QueryResult;

/// Row trait for accessing individual result rows
pub use backend::Row;

/// Value trait for type-safe value extraction from rows
pub use backend::Value;

/// Type-safe query parameter container
pub use backend::QueryParams;

/// Parameter value types (String, Int, Float, Bool)
pub use backend::ValueType;

// ============================================================================
// Database Operations
// ============================================================================

/// Open a database connection at the specified path
pub use db::open_db;

/// Execute a query with parameters
pub use db::run_query;

/// Execute a query without parameters (convenience wrapper)
pub use db::run_query_no_params;

/// Database error type
pub use db::DbError;

/// Try to create a database relation, returning Ok(false) if it already exists
pub use db::try_create_relation;

/// Open an in-memory database for testing
#[cfg(any(test, feature = "test-utils"))]
pub use db::open_mem_db;

// ============================================================================
// Value Extraction Helpers
// ============================================================================

/// Extract a string value from a database Value
pub use db::extract_string;

/// Extract an i64 value from a database Value
pub use db::extract_i64;

/// Extract an f64 value from a database Value
pub use db::extract_f64;

/// Extract a boolean value from a database Value
pub use db::extract_bool;

/// Extract a string value with a default fallback
pub use db::extract_string_or;

// ============================================================================
// Call Graph Extraction
// ============================================================================

/// Layout description for extracting Call objects from query rows
pub use db::CallRowLayout;

/// Extract a Call from a row using the Database trait (backend-agnostic)
pub use db::extract_call_from_row_trait;

/// Extract a Call from a CozoDB DataValue row (CozoDB-specific)
#[cfg(feature = "backend-cozo")]
pub use db::extract_call_from_row;

// ============================================================================
// Query Building Helpers
// ============================================================================

/// Escape a string for use in double-quoted string literals
pub use db::escape_string;

/// Escape a string for use in single-quoted string literals
pub use db::escape_string_single;

// ============================================================================
// Domain Types
// ============================================================================

/// A function call relationship between caller and callee
pub use types::Call;

/// Reference to a function (module, name, arity)
pub use types::FunctionRef;

/// A group of modules with associated metadata
pub use types::ModuleGroup;

/// Result containing grouped module data
pub use types::ModuleGroupResult;

/// Collection of modules with metadata
pub use types::ModuleCollectionResult;

/// Trace/path result between functions
pub use types::TraceResult;

/// Single entry in a trace path
pub use types::TraceEntry;

/// Direction of trace (forward or reverse)
pub use types::TraceDirection;

/// Shared string type for efficient string handling
pub use types::SharedStr;

// ============================================================================
// Query Builders
// ============================================================================

/// Builder for constructing SQL WHERE conditions
pub use query_builders::ConditionBuilder;

/// Builder for optional WHERE conditions
pub use query_builders::OptionalConditionBuilder;

/// Validate a single regex pattern
pub use query_builders::validate_regex_pattern;

/// Validate multiple regex patterns
pub use query_builders::validate_regex_patterns;

// ============================================================================
// Backend-Specific Exports (Deprecated)
// ============================================================================

/// CozoDB's DbInstance type (deprecated - use Box<dyn Database> instead)
///
/// This export is provided for backward compatibility but is deprecated.
/// New code should use the `Database` trait instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `Box<dyn Database>` instead of `DbInstance` for backend abstraction"
)]
#[cfg(feature = "backend-cozo")]
pub use cozo::DbInstance;
