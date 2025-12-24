//! Database layer for code search - CozoDB queries and call graph data structures

pub mod backend;
pub mod db;
pub mod types;
pub mod query_builders;
pub mod queries;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

#[cfg(any(test, feature = "test-utils"))]
pub mod fixtures;

// Re-export commonly used items
pub use db::{
    open_db, run_query, run_query_no_params, DbError,
    extract_call_from_row_trait,
    extract_string, extract_i64, extract_f64,
    extract_bool, extract_string_or, CallRowLayout,
    try_create_relation,
};

// CozoDB-specific exports
#[cfg(feature = "backend-cozo")]
pub use db::extract_call_from_row;

#[cfg(feature = "backend-cozo")]
pub use cozo::DbInstance;

// Backend abstraction exports
pub use backend::{Database, QueryResult, Row, Value, QueryParams};

#[cfg(any(test, feature = "test-utils"))]
pub use db::open_mem_db;

pub use types::{
    Call, FunctionRef, ModuleGroup, ModuleGroupResult,
    ModuleCollectionResult, TraceResult, TraceEntry,
    TraceDirection, SharedStr
};

pub use query_builders::{ConditionBuilder, OptionalConditionBuilder, validate_regex_pattern, validate_regex_patterns};
