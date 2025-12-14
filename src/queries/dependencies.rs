//! Unified module dependency queries.
//!
//! This module provides a single query function that can find dependencies in either direction:
//! - `Outgoing`: Find modules that the matched module depends ON (imports/calls into)
//! - `Incoming`: Find modules that depend on (are depended BY) the matched module

use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use thiserror::Error;

use crate::db::{extract_call_from_row, run_query, CallRowLayout, Params};
use crate::types::Call;

#[derive(Error, Debug)]
pub enum DependencyError {
    #[error("Dependency query failed: {message}")]
    QueryFailed { message: String },
}

/// Direction of dependency analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyDirection {
    /// Find modules that the matched module depends ON (outgoing dependencies)
    /// Query: "What does module X call?"
    Outgoing,
    /// Find modules that depend on the matched module (incoming dependencies)
    /// Query: "Who calls module X?"
    Incoming,
}

impl DependencyDirection {
    /// Returns the field name to filter on based on direction
    fn filter_field(&self) -> &'static str {
        match self {
            DependencyDirection::Outgoing => "caller_module",
            DependencyDirection::Incoming => "callee_module",
        }
    }

    /// Returns the ORDER BY clause based on direction
    fn order_clause(&self) -> &'static str {
        match self {
            DependencyDirection::Outgoing => {
                "callee_module, callee_function, callee_arity, caller_module, caller_name, caller_arity, call_line"
            }
            DependencyDirection::Incoming => {
                "caller_module, caller_name, caller_arity, callee_function, callee_arity, call_line"
            }
        }
    }
}

/// Find module dependencies in the specified direction.
///
/// - `Outgoing`: Returns calls from the matched module to other modules
/// - `Incoming`: Returns calls from other modules to the matched module
///
/// Self-references (calls within the same module) are excluded.
pub fn find_dependencies(
    db: &dyn DatabaseBackend,
    direction: DependencyDirection,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    let filter_field = direction.filter_field();
    let order_clause = direction.order_clause();

    // Build module condition using the appropriate field name
    let module_cond =
        crate::utils::ConditionBuilder::new(filter_field, "module_pattern").build(use_regex);

    // Query calls with function_locations join for caller metadata, excluding self-references
    // Filter out struct calls (callee_function != '%')
    let script = format!(
        r#"
        ?[caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            callee_function != '%',
            {module_cond},
            caller_module != callee_module,
            project == $project
        :order {order_clause}
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert(
        "module_pattern".to_string(),
        DataValue::Str(module_pattern.into()),
    );
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| DependencyError::QueryFailed {
        message: e.to_string(),
    })?;

    let layout = CallRowLayout::from_headers(&rows.headers)?;
    let results = rows
        .rows
        .iter()
        .filter_map(|row| extract_call_from_row(row, &layout))
        .collect();

    Ok(results)
}
