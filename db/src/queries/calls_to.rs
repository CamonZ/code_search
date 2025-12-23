//! Find incoming calls to functions.
//!
//! This is a convenience wrapper around [`super::calls::find_calls`] with
//! [`CallDirection::To`](super::calls::CallDirection::To).

use std::error::Error;

use super::calls::{find_calls, CallDirection};
use crate::types::Call;

pub fn find_calls_to(
    db: &cozo::DbInstance,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    find_calls(
        db,
        CallDirection::To,
        module_pattern,
        function_pattern,
        arity,
        project,
        use_regex,
        limit,
    )
}
