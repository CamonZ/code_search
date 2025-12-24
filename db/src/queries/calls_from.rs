//! Find outgoing calls from functions.
//!
//! This is a convenience wrapper around [`super::calls::find_calls`] with
//! [`CallDirection::From`](super::calls::CallDirection::From).

use std::error::Error;

use super::calls::{find_calls, CallDirection};
use crate::backend::Database;
use crate::types::Call;

pub fn find_calls_from(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    find_calls(
        db,
        CallDirection::From,
        module_pattern,
        function_pattern,
        arity,
        project,
        use_regex,
        limit,
    )
}
