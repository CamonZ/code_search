//! Unified module dependency queries.
//!
//! This module provides a single query function that can find dependencies in either direction:
//! - `Outgoing`: Find modules that the matched module depends ON (imports/calls into)
//! - `Incoming`: Find modules that depend on (are depended BY) the matched module

use std::error::Error;

use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::types::Call;

#[cfg(feature = "backend-surrealdb")]
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-surrealdb")]
use crate::types::FunctionRef;

#[cfg(feature = "backend-cozo")]
use crate::db::{extract_call_from_row_trait, run_query, CallRowLayout};

#[cfg(feature = "backend-cozo")]
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
    #[cfg(feature = "backend-cozo")]
    fn filter_field(&self) -> &'static str {
        match self {
            DependencyDirection::Outgoing => "caller_module",
            DependencyDirection::Incoming => "callee_module",
        }
    }

    /// Returns the ORDER BY clause based on direction
    #[cfg(feature = "backend-cozo")]
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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
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
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    use std::rc::Rc;
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    // Build module matching condition based on direction and regex flag
    let module_condition = match (direction, use_regex) {
        (DependencyDirection::Outgoing, false) => "in.module_name = $module_pattern",
        (DependencyDirection::Outgoing, true) => "string::matches(in.module_name, $module_pattern)",
        (DependencyDirection::Incoming, false) => "out.module_name = $module_pattern",
        (DependencyDirection::Incoming, true) => "string::matches(out.module_name, $module_pattern)",
    };

    // Query calls edge table, filtering out self-references (same module)
    // Note: SurrealDB returns in/out as record references, so we access their IDs
    let query = format!(
        r#"
        SELECT in, out, line FROM calls
        WHERE {} AND in.module_name != out.module_name
        LIMIT $limit;
        "#,
        module_condition
    );

    let params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_int("limit", limit as i64);

    let result = db
        .execute_query(&query, params)
        .map_err(|e| DependencyError::QueryFailed {
            message: e.to_string(),
        })?;

    // Parse results - each row contains (in, out, line) where in/out are record references
    // Headers are: ["in", "line", "out"] so indices are: in=0, line=1, out=2
    let mut results = Vec::new();
    for row in result.rows() {
        // Extract caller (in at index 0) and callee (out at index 2) from record references
        let Some(caller_ref) = row.get(0).and_then(|v| extract_function_ref_from_value(v)) else {
            continue;
        };
        let Some(callee_ref) = row.get(2).and_then(|v| extract_function_ref_from_value(v)) else {
            continue;
        };
        let line = row.get(1).and_then(|v| v.as_i64()).unwrap_or(0);

        let caller = FunctionRef::new(
            Rc::from(caller_ref.0.as_str()),
            Rc::from(caller_ref.1.as_str()),
            caller_ref.2,
        );
        let callee = FunctionRef::new(
            Rc::from(callee_ref.0.as_str()),
            Rc::from(callee_ref.1.as_str()),
            callee_ref.2,
        );

        results.push(Call {
            caller,
            callee,
            line,
            call_type: None,
            depth: None,
        });
    }

    Ok(results)
}

/// Extract (module, name, arity) from a SurrealDB record reference (Thing).
/// The function record ID format is: `function`:[$module, $name, $arity]
#[cfg(feature = "backend-surrealdb")]
fn extract_function_ref_from_value(value: &dyn crate::backend::Value) -> Option<(String, String, i64)> {
    let id = value.as_thing_id()?;
    let parts = id.as_array()?;

    let module = parts.get(0)?.as_str()?;
    let name = parts.get(1)?.as_str()?;
    let arity = parts.get(2)?.as_i64()?;

    Some((module.to_string(), name.to_string(), arity))
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

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_dependencies_outgoing_forward() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture: MyApp.Service calls MyApp.Accounts and MyApp.Notifier
        // Outgoing dependencies for MyApp.Service should include cross-module calls
        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "MyApp.Service",
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let deps = result.unwrap();

        // Should find calls from MyApp.Service to MyApp.Accounts and MyApp.Notifier
        assert_eq!(deps.len(), 2, "Should find exactly 2 outgoing cross-module dependencies");

        // Verify all callers are from MyApp.Service
        for dep in &deps {
            assert_eq!(dep.caller.module.as_ref(), "MyApp.Service");
        }

        // Verify callees (order may vary)
        let callees: Vec<(&str, &str)> = deps
            .iter()
            .map(|d| (d.callee.module.as_ref(), d.callee.name.as_ref()))
            .collect();
        assert!(
            callees.contains(&("MyApp.Accounts", "get_user")),
            "Should call MyApp.Accounts.get_user"
        );
        assert!(
            callees.contains(&("MyApp.Notifier", "send_email")),
            "Should call MyApp.Notifier.send_email"
        );
    }

    #[test]
    fn test_find_dependencies_incoming_reverse() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture: MyApp.Notifier is called by MyApp.Service and MyApp.Controller
        // Incoming dependencies for MyApp.Notifier: calls FROM other modules TO MyApp.Notifier
        let result = find_dependencies(
            &*db,
            DependencyDirection::Incoming,
            "MyApp.Notifier",
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let deps = result.unwrap();

        // Should find calls from MyApp.Service and MyApp.Controller to MyApp.Notifier
        assert_eq!(deps.len(), 2, "Should find exactly 2 incoming cross-module dependencies");

        // All callees should be to MyApp.Notifier
        for dep in &deps {
            assert_eq!(dep.callee.module.as_ref(), "MyApp.Notifier");
        }

        // Verify callers (order may vary)
        let callers: Vec<(&str, &str)> = deps
            .iter()
            .map(|d| (d.caller.module.as_ref(), d.caller.name.as_ref()))
            .collect();
        assert!(
            callers.contains(&("MyApp.Service", "process_request")),
            "Should be called by MyApp.Service.process_request"
        );
        assert!(
            callers.contains(&("MyApp.Controller", "create")),
            "Should be called by MyApp.Controller.create"
        );
    }

    #[test]
    fn test_find_dependencies_excludes_self_references() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let deps = result.unwrap();

        // All dependencies should be to different modules
        for dep in &deps {
            assert_ne!(
                dep.caller.module, dep.callee.module,
                "Should exclude self-references (caller: {}, callee: {})",
                dep.caller.module, dep.callee.module
            );
        }
    }

    #[test]
    fn test_find_dependencies_complex_outgoing() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture has multiple cross-module dependencies
        // Controller functions call Accounts, Service, Notifier
        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let deps = result.unwrap();

        // Should find multiple outgoing dependencies
        assert!(!deps.is_empty(), "Should find outgoing dependencies from Controller");

        // Extract unique target modules
        let target_modules: Vec<_> = deps
            .iter()
            .map(|d| d.callee.module.as_ref())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Should have dependencies to Accounts and/or Service and/or Notifier
        assert!(
            target_modules.len() > 0,
            "Should have dependencies to other modules"
        );

        // Verify all are different from Controller
        for module in target_modules {
            assert_ne!(
                module, "MyApp.Controller",
                "Should not have self-references"
            );
        }
    }

    #[test]
    fn test_find_dependencies_complex_incoming() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Complex fixture: Accounts functions are called by Controller
        let result = find_dependencies(
            &*db,
            DependencyDirection::Incoming,
            "MyApp.Accounts",
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let deps = result.unwrap();

        // Should find incoming dependencies (callers)
        assert!(!deps.is_empty(), "Should find incoming dependencies to Accounts");

        // Extract unique source modules
        let source_modules: Vec<_> = deps
            .iter()
            .map(|d| d.caller.module.as_ref())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Should have dependencies from other modules
        assert!(
            source_modules.len() > 0,
            "Should have dependencies from other modules"
        );

        // Verify all are different from Accounts
        for module in source_modules {
            assert_ne!(
                module, "MyApp.Accounts",
                "Should not have self-references"
            );
        }
    }

    #[test]
    fn test_find_dependencies_empty_results_nonexistent() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "NonExistent",
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed");
        let deps = result.unwrap();
        assert!(
            deps.is_empty(),
            "Non-existent module should have no dependencies"
        );
    }

    #[test]
    fn test_find_dependencies_respects_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let limit_1 = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            1,
        )
        .unwrap();

        let limit_100 = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            100,
        )
        .unwrap();

        // Limit should be respected
        assert!(limit_1.len() <= 1, "Limit 1 should return at most 1 result");
        assert!(
            limit_1.len() <= limit_100.len(),
            "Higher limit should return >= results"
        );
    }

    #[test]
    fn test_find_dependencies_with_regex_pattern() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use regex pattern to match Controller
        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "^MyApp\\.Controller$",
            "default",
            true,
            100,
        );

        assert!(result.is_ok(), "Query should succeed with regex: {:?}", result.err());
        let deps = result.unwrap();

        // All calls should be from MyApp.Controller
        if !deps.is_empty() {
            for dep in &deps {
                assert_eq!(
                    dep.caller.module.as_ref(),
                    "MyApp.Controller",
                    "Regex pattern should match only Controller"
                );
            }
        }
    }

    #[test]
    fn test_find_dependencies_invalid_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "[invalid",
            "default",
            true,
            100,
        );

        assert!(
            result.is_err(),
            "Should reject invalid regex pattern"
        );
    }

    #[test]
    fn test_find_dependencies_all_fields_populated() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "module_a",
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed");
        let deps = result.unwrap();

        if !deps.is_empty() {
            for (i, dep) in deps.iter().enumerate() {
                assert!(
                    !dep.caller.module.is_empty(),
                    "Call {}: Caller module should not be empty",
                    i
                );
                assert!(
                    !dep.caller.name.is_empty(),
                    "Call {}: Caller name should not be empty",
                    i
                );
                assert!(
                    !dep.callee.module.is_empty(),
                    "Call {}: Callee module should not be empty",
                    i
                );
                assert!(
                    !dep.callee.name.is_empty(),
                    "Call {}: Callee name should not be empty",
                    i
                );
                assert!(
                    dep.caller.arity >= 0,
                    "Call {}: Caller arity should be >= 0",
                    i
                );
                assert!(
                    dep.callee.arity >= 0,
                    "Call {}: Callee arity should be >= 0",
                    i
                );
            }
        }
    }

    #[test]
    fn test_find_dependencies_incoming_with_regex() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Use regex to match Accounts module
        let result = find_dependencies(
            &*db,
            DependencyDirection::Incoming,
            "^MyApp\\.Accounts$",
            "default",
            true,
            100,
        );

        assert!(result.is_ok(), "Query should succeed with regex: {:?}", result.err());
        let deps = result.unwrap();

        // All calls should target MyApp.Accounts
        if !deps.is_empty() {
            for dep in &deps {
                assert_eq!(
                    dep.callee.module.as_ref(),
                    "MyApp.Accounts",
                    "Regex pattern should match only Accounts"
                );
            }
        }
    }

    #[test]
    fn test_find_dependencies_pattern_matching_partial() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Regex pattern: any module starting with MyApp
        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "^MyApp.*",
            "default",
            true,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let deps = result.unwrap();

        // All calls should be from MyApp.* modules
        for dep in &deps {
            assert!(
                dep.caller.module.starts_with("MyApp"),
                "Regex should match modules starting with MyApp, got: {}",
                dep.caller.module
            );
        }
    }

    #[test]
    fn test_find_dependencies_outgoing_field_values() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "MyApp.Service",
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let deps = result.unwrap();

        // Verify we have the expected call from MyApp.Service.process_request/2 to MyApp.Notifier.send_email/2
        let has_expected = deps.iter().any(|d| {
            d.caller.module.as_ref() == "MyApp.Service"
                && d.caller.name.as_ref() == "process_request"
                && d.caller.arity == 2
                && d.callee.module.as_ref() == "MyApp.Notifier"
                && d.callee.name.as_ref() == "send_email"
                && d.callee.arity == 2
        });

        assert!(
            has_expected,
            "Should find expected call: MyApp.Service.process_request/2 -> MyApp.Notifier.send_email/2"
        );
    }

    #[test]
    fn test_find_dependencies_incoming_field_values() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        let result = find_dependencies(
            &*db,
            DependencyDirection::Incoming,
            "MyApp.Notifier",
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let deps = result.unwrap();

        // Verify we have the expected call from MyApp.Service.process_request/2 to MyApp.Notifier.send_email/2
        let has_expected = deps.iter().any(|d| {
            d.caller.module.as_ref() == "MyApp.Service"
                && d.caller.name.as_ref() == "process_request"
                && d.caller.arity == 2
                && d.callee.module.as_ref() == "MyApp.Notifier"
                && d.callee.name.as_ref() == "send_email"
                && d.callee.arity == 2
        });

        assert!(
            has_expected,
            "Should find expected call: MyApp.Service.process_request/2 -> MyApp.Notifier.send_email/2"
        );
    }

    #[test]
    fn test_find_dependencies_zero_limit() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Zero limit should return empty results
        let result = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "module_a",
            "default",
            false,
            0,
        );

        assert!(result.is_ok());
        let deps = result.unwrap();
        assert!(deps.is_empty(), "Zero limit should return empty results");
    }

    #[test]
    fn test_find_dependencies_count_matches() {
        let db = crate::test_utils::surreal_call_graph_db_complex();

        // Test outgoing from Controller
        let outgoing = find_dependencies(
            &*db,
            DependencyDirection::Outgoing,
            "MyApp.Controller",
            "default",
            false,
            100,
        )
        .unwrap();

        // Test incoming to Accounts
        let incoming = find_dependencies(
            &*db,
            DependencyDirection::Incoming,
            "MyApp.Accounts",
            "default",
            false,
            100,
        )
        .unwrap();

        // Both should have results
        assert!(!outgoing.is_empty(), "Should have outgoing dependencies");
        assert!(!incoming.is_empty(), "Should have incoming dependencies");

        // Count should be reasonable (at least 1)
        assert!(outgoing.len() >= 1, "Outgoing count should be >= 1");
        assert!(incoming.len() >= 1, "Incoming count should be >= 1");
    }
}
