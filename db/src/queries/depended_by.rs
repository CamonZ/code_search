//! Find incoming module dependencies.
//!
//! This is a convenience wrapper around [`super::dependencies::find_dependencies`] with
//! [`DependencyDirection::Incoming`](super::dependencies::DependencyDirection::Incoming).

use std::error::Error;

use super::dependencies::{find_dependencies as query_dependencies, DependencyDirection};
use crate::backend::Database;
use crate::types::Call;

pub fn find_dependents(
    db: &dyn Database,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    query_dependencies(
        db,
        DependencyDirection::Incoming,
        module_pattern,
        project,
        use_regex,
        limit,
    )
}
