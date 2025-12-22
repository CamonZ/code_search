//! Test fixtures for execute tests.
//!
//! This module provides JSON fixtures for testing commands. Fixtures are
//! loaded at compile time using `include_str!` for zero runtime overhead.
//!
//! ## Available Fixtures
//!
//! - [`CALL_GRAPH`] - Function locations and call relationships
//! - [`TYPE_SIGNATURES`] - Function type signatures
//! - [`STRUCTS`] - Struct definitions with fields
//!
//! ## Usage
//!
//! ```ignore
//! use crate::fixtures;
//!
//! crate::execute_test_fixture! {
//!     fixture_name: populated_db,
//!     json: fixtures::CALL_GRAPH,
//!     project: "test_project",
//! }
//! ```

/// Call graph fixture with function locations and call relationships.
///
/// Contains:
/// - 5 modules: Controller, Accounts, Service, Repo, Notifier
/// - 15 functions with various arities and kinds (def/defp)
/// - 11 call edges forming a realistic call graph
///
/// Use for: trace, reverse_trace, calls_from, calls_to, path, hotspots,
/// unused, depends_on, depended_by
pub const CALL_GRAPH: &str = include_str!("call_graph.json");

/// Type signatures fixture with function specs.
///
/// Contains:
/// - 3 modules: Accounts, Users, Repo
/// - 9 function signatures with typed arguments and return types
///
/// Use for: search (functions kind), function
pub const TYPE_SIGNATURES: &str = include_str!("type_signatures.json");

/// Struct definitions fixture.
///
/// Contains:
/// - 3 structs: User, Post, Comment
/// - Various field types with defaults and required flags
///
/// Use for: struct command
pub const STRUCTS: &str = include_str!("structs.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_graph_is_valid_json() {
        let _: serde_json::Value = serde_json::from_str(CALL_GRAPH)
            .expect("CALL_GRAPH should be valid JSON");
    }

    #[test]
    fn test_type_signatures_is_valid_json() {
        let _: serde_json::Value = serde_json::from_str(TYPE_SIGNATURES)
            .expect("TYPE_SIGNATURES should be valid JSON");
    }

    #[test]
    fn test_structs_is_valid_json() {
        let _: serde_json::Value = serde_json::from_str(STRUCTS)
            .expect("STRUCTS should be valid JSON");
    }
}
