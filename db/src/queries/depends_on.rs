//! Find outgoing module dependencies.
//!
//! This is a convenience wrapper around [`super::dependencies::find_dependencies`] with
//! [`DependencyDirection::Outgoing`](super::dependencies::DependencyDirection::Outgoing).

use std::error::Error;

use super::dependencies::{find_dependencies as query_dependencies, DependencyDirection};
use crate::backend::Database;
use crate::types::Call;

pub fn find_dependencies(
    db: &dyn Database,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    query_dependencies(
        db,
        DependencyDirection::Outgoing,
        module_pattern,
        project,
        use_regex,
        limit,
    )
}
