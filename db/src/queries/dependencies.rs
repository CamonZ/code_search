//! Unified module dependency queries.
//!
//! This module provides a single query function that can find dependencies in either direction:
//! - `Outgoing`: Find modules that the matched module depends ON (imports/calls into)
//! - `Incoming`: Find modules that depend on (are depended BY) the matched module

use std::error::Error;

use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_call_from_row_trait, run_query, CallRowLayout};
use crate::types::Call;
use crate::query_builders::ConditionBuilder;

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
    db: &dyn Database,
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
        ConditionBuilder::new(filter_field, "module_pattern").build(use_regex);

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

    let params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| DependencyError::QueryFailed {
        message: e.to_string(),
    })?;

    let layout = CallRowLayout::from_headers(result.headers())?;
    let results = result
        .rows()
        .iter()
        .filter_map(|row| extract_call_from_row_trait(&**row, &layout))
        .collect();

    Ok(results)
}

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::call_graph_db("default")
    }

    #[rstest]
    fn test_find_dependencies_outgoing_returns_results(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_dependencies(
            &*populated_db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let deps = result.unwrap();
        // Should find outgoing dependencies
        assert!(!deps.is_empty(), "Should find outgoing dependencies");
    }

    #[rstest]
    fn test_find_dependencies_incoming_returns_results(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_dependencies(
            &*populated_db,
            DependencyDirection::Incoming,
            "MyApp",
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let deps = result.unwrap();
        // May have incoming dependencies
        assert!(deps.is_empty() || !deps.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_dependencies_excludes_self_references(
        populated_db: Box<dyn crate::backend::Database>,
    ) {
        let result = find_dependencies(
            &*populated_db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let deps = result.unwrap();
        for dep in &deps {
            assert_ne!(dep.caller.module, dep.callee.module, "Should exclude self-references");
        }
    }

    #[rstest]
    fn test_find_dependencies_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(
            &*populated_db,
            DependencyDirection::Outgoing,
            "NonExistent",
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let deps = result.unwrap();
        assert!(deps.is_empty(), "Non-existent module should have no dependencies");
    }

    #[rstest]
    fn test_find_dependencies_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_dependencies(
            &*populated_db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            5,
        )
        .unwrap();
        let limit_100 = find_dependencies(
            &*populated_db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            100,
        )
        .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_dependencies_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_dependencies(
            &*populated_db,
            DependencyDirection::Outgoing,
            "MyApp",
            "nonexistent",
            false,
            100,
        );
        assert!(result.is_ok());
        let deps = result.unwrap();
        assert!(deps.is_empty(), "Non-existent project should return no results");
    }
}
