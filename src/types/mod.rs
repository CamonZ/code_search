//! Shared types for call graph data.

mod call;
mod results;
mod trace;

pub use call::{Call, FunctionRef};
pub use results::{ModuleGroupResult, ModuleCollectionResult, ModuleGroup};
pub use trace::{TraceDirection, TraceEntry, TraceResult};
