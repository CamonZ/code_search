//! Database query modules for call graph analysis.
//!
//! Each module contains CozoScript queries and result parsing for a specific
//! command. Queries execute against a CozoDB instance and return typed results.
//!
//! # Query Categories
//!
//! ## Data Import
//! - [`import`] - Import JSON call graph data into database relations
//!
//! ## Basic Lookups
//! - [`location`] - Find function definition locations by name
//! - [`function`] - Get function signatures with type information
//! - [`search`] - Full-text search across functions, specs, and types
//! - [`file`] - List all functions defined in a module/file
//!
//! ## Call Graph Traversal
//! - [`calls_from`] - Find all functions called by a given function
//! - [`calls_to`] - Find all callers of a given function
//! - [`trace`] - Forward call trace to specified depth
//! - [`reverse_trace`] - Backward call trace (who calls this, recursively)
//! - [`path`] - Find call path between two functions
//!
//! ## Dependency Analysis
//! - [`depends_on`] - Modules that a given module depends on
//! - [`depended_by`] - Modules that depend on a given module
//!
//! ## Code Quality
//! - [`unused`] - Find functions that are never called
//! - [`hotspots`] - Find most-called functions (high fan-in)
//!
//! ## Type System
//! - [`specs`] - Query @spec and @callback definitions
//! - [`types`] - Query @type, @typep, and @opaque definitions
//! - [`structs`] - Query struct definitions with field info
//!
//! # Performance
//!
//! All queries are indexed by module/function names. Most lookups are O(log n)
//! with O(m) result iteration where m is result count. Trace queries may be
//! O(n * depth) in worst case for highly connected graphs.
//!
//! # Query Pattern
//!
//! Each query module exports a single `find_*` or `*_query` function that:
//! 1. Builds a CozoScript query string with interpolated parameters
//! 2. Executes via `db.run_script()`
//! 3. Extracts results into typed Rust structs
//!
//! Parameters are escaped using [`crate::db::escape_string`] to prevent injection.

pub mod accepts;
pub mod calls;
pub mod calls_from;
pub mod calls_to;
pub mod clusters;
pub mod complexity;
pub mod cycles;
pub mod depended_by;
pub mod dependencies;
pub mod depends_on;
pub mod duplicates;
pub mod file;
pub mod function;
pub mod hotspots;
pub mod import;
pub mod import_models;
pub mod large_functions;
pub mod location;
pub mod many_clauses;
pub mod path;
pub mod returns;
pub mod reverse_trace;
pub mod schema;
pub mod search;
pub mod specs;
pub mod struct_usage;
pub mod structs;
pub mod trace;
pub mod types;
pub mod unused;