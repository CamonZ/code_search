//! Shared types for call graph data.

use std::rc::Rc;

mod call;
mod results;
mod trace;

pub use call::{Call, FunctionRef};
pub use results::{ModuleGroupResult, ModuleCollectionResult, ModuleGroup};
pub use trace::{TraceDirection, TraceEntry, TraceResult};

/// Type alias for shared, reference-counted strings.
/// Used throughout FunctionRef and Call structures to reduce memory allocations
/// when the same module/function names appear multiple times.
pub type SharedStr = Rc<str>;
