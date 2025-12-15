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
use crate::queries::builder::{QueryBuilder, CompiledQuery};

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

/// Query builder for finding module dependencies
#[derive(Debug)]
pub struct DependenciesQueryBuilder {
    pub direction: DependencyDirection,
    pub module_pattern: String,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for DependenciesQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("module_pattern".to_string(), DataValue::Str(self.module_pattern.clone().into()));
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        params
    }
}

impl DependenciesQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let filter_field = self.direction.filter_field();
        let order_clause = self.direction.order_clause();

        // Build module condition using the appropriate field name
        let module_cond =
            crate::utils::ConditionBuilder::new(filter_field, "module_pattern").build(self.use_regex);

        // Query calls with function_locations join for caller metadata, excluding self-references
        // Filter out struct calls (callee_function != '%')
        Ok(format!(
            r#"?[caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
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
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices, not edges:
        // - Call vertex: caller_module, caller_function, callee_module, callee_function, etc.
        // - FunctionLocation vertex: module, name, arity, start_line, end_line, kind, etc.
        // We join on properties rather than using edge relationships.

        let mod_match = if self.use_regex { "=~" } else { "=" };

        // Build WHERE conditions based on direction
        let (module_filter, order_clause) = match self.direction {
            DependencyDirection::Outgoing => {
                (format!("c.caller_module {} $module_pattern", mod_match),
                 "c.callee_module, c.callee_function, c.callee_arity")
            }
            DependencyDirection::Incoming => {
                (format!("c.callee_module {} $module_pattern", mod_match),
                 "c.caller_module, loc.name, loc.arity")
            }
        };

        Ok(format!(
            r#"MATCH (c:Call), (loc:FunctionLocation)
WHERE c.project = $project
  AND {}
  AND c.caller_module <> c.callee_module
  AND c.callee_function <> '%'
  AND loc.module = c.caller_module
  AND c.caller_function STARTS WITH loc.name
  AND c.line >= loc.start_line
  AND c.line <= loc.end_line
RETURN c.caller_module, loc.name AS caller_name, loc.arity AS caller_arity,
       loc.kind AS caller_kind, loc.start_line AS caller_start_line, loc.end_line AS caller_end_line,
       c.callee_module, c.callee_function, c.callee_arity,
       c.file, c.line AS call_line
ORDER BY {}
LIMIT {}"#,
            module_filter, order_clause, self.limit
        ))
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
    let builder = DependenciesQueryBuilder {
        direction,
        module_pattern: module_pattern.to_string(),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| DependencyError::QueryFailed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_dependencies_query_cozo_outgoing() {
        let builder = DependenciesQueryBuilder {
            direction: DependencyDirection::Outgoing,
            module_pattern: "MyApp.Server".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("*calls"));
        assert!(compiled.contains("*function_locations"));
        assert!(compiled.contains("caller_module != callee_module"));
        assert!(compiled.contains("caller_module"));
    }

    #[test]
    fn test_dependencies_query_cozo_incoming() {
        let builder = DependenciesQueryBuilder {
            direction: DependencyDirection::Incoming,
            module_pattern: "MyApp.Server".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("callee_module"));
        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_dependencies_query_age_outgoing() {
        let builder = DependenciesQueryBuilder {
            direction: DependencyDirection::Outgoing,
            module_pattern: "MyApp".to_string(),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (c:Call), (loc:FunctionLocation)"));
        assert!(compiled.contains("c.caller_module =~"));
        assert!(compiled.contains("c.caller_module <> c.callee_module"));
    }

    #[test]
    fn test_dependencies_query_age_incoming() {
        let builder = DependenciesQueryBuilder {
            direction: DependencyDirection::Incoming,
            module_pattern: "MyApp".to_string(),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (c:Call), (loc:FunctionLocation)"));
        assert!(compiled.contains("c.callee_module = $module_pattern"));
    }

    #[test]
    fn test_dependencies_query_parameters() {
        let builder = DependenciesQueryBuilder {
            direction: DependencyDirection::Outgoing,
            module_pattern: "mod".to_string(),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("module_pattern"));
        assert!(params.contains_key("project"));
    }

    #[test]
    fn test_dependencies_direction_enum() {
        assert_eq!(DependencyDirection::Outgoing.filter_field(), "caller_module");
        assert_eq!(DependencyDirection::Incoming.filter_field(), "callee_module");
    }
}
