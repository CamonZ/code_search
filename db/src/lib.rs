//! Database layer for code search - CozoDB queries and call graph data structures

pub mod db;
pub mod types;
pub mod query_builders;
pub mod queries;

// Re-export commonly used items
pub use db::{open_db, run_query, run_query_no_params, DbError, Params};

#[cfg(test)]
pub use db::open_mem_db;

pub use types::{
    Call, FunctionRef, ModuleGroup, ModuleGroupResult,
    ModuleCollectionResult, TraceResult, TraceEntry,
    TraceDirection, SharedStr
};

pub use query_builders::{ConditionBuilder, OptionalConditionBuilder};
