//! Unified call graph queries for finding function calls.
//!
//! This module provides a single query function that can find calls in either direction:
//! - `From`: Find all calls made BY the matched functions (outgoing calls)
//! - `To`: Find all calls made TO the matched functions (incoming calls)

use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use thiserror::Error;

use crate::db::{extract_call_from_row, run_query, CallRowLayout, Params};
use crate::types::Call;
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum CallsError {
    #[error("Calls query failed: {message}")]
    QueryFailed { message: String },
}

/// Direction of call graph traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    /// Find calls FROM the matched functions (what does this function call?)
    From,
    /// Find calls TO the matched functions (who calls this function?)
    To,
}

impl CallDirection {
    /// Returns the field names to filter on based on direction
    fn filter_fields(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            CallDirection::From => ("caller_module", "caller_name", "caller_arity"),
            CallDirection::To => ("callee_module", "callee_function", "callee_arity"),
        }
    }

    /// Returns the ORDER BY clause based on direction
    fn order_clause(&self) -> &'static str {
        match self {
            CallDirection::From => {
                "caller_module, caller_name, caller_arity, call_line, callee_module, callee_function, callee_arity"
            }
            CallDirection::To => {
                "callee_module, callee_function, callee_arity, caller_module, caller_name, caller_arity"
            }
        }
    }
}

/// Query builder for finding function calls
#[derive(Debug)]
pub struct CallsQueryBuilder {
    pub direction: CallDirection,
    pub module_pattern: String,
    pub function_pattern: Option<String>,
    pub arity: Option<i64>,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for CallsQueryBuilder {
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
        if let Some(ref fn_pat) = self.function_pattern {
            params.insert("function_pattern".to_string(), DataValue::Str(fn_pat.clone().into()));
        }
        if let Some(a) = self.arity {
            params.insert("arity".to_string(), DataValue::from(a));
        }
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        params
    }
}

impl CallsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let (module_field, function_field, arity_field) = self.direction.filter_fields();
        let order_clause = self.direction.order_clause();

        // Build conditions using the appropriate field names
        let module_cond =
            crate::utils::ConditionBuilder::new(module_field, "module_pattern").build(self.use_regex);
        let function_cond =
            crate::utils::OptionalConditionBuilder::new(function_field, "function_pattern")
                .with_leading_comma()
                .with_regex()
                .build_with_regex(self.function_pattern.is_some(), self.use_regex);
        let arity_cond = crate::utils::OptionalConditionBuilder::new(arity_field, "arity")
            .with_leading_comma()
            .build(self.arity.is_some());

        let project_cond = ", project == $project";

        // Join calls with function_locations to get caller's arity and line range
        // Filter out struct calls (callee_function == '%')
        Ok(format!(
            r#"?[project, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line, call_type] :=
    *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line, call_type, caller_kind}},
    *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, start_line: caller_start_line, end_line: caller_end_line}},
    starts_with(caller_function, caller_name),
    call_line >= caller_start_line,
    call_line <= caller_end_line,
    callee_function != '%',
    {module_cond}
    {function_cond}
    {arity_cond}
    {project_cond}
:order {order_clause}
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices, not edges:
        // - Call vertex: caller_module, caller_function, callee_module, callee_function, etc.
        // - FunctionLocation vertex: module, function, arity, start_line, end_line, etc.
        // We join on properties rather than using edge relationships.

        let mod_match = if self.use_regex { "=~" } else { "=" };
        let fn_match = if self.use_regex { "=~" } else { "=" };

        // Determine which fields to filter on based on direction
        // Note: FunctionLocation uses 'name' for function name, not 'function'
        let (module_field, function_field, arity_field) = match self.direction {
            CallDirection::From => ("c.caller_module", "loc.name", "loc.arity"),
            CallDirection::To => ("c.callee_module", "c.callee_function", "c.callee_arity"),
        };

        // Build WHERE conditions
        let mut where_conditions = vec![
            "c.project = $project".to_string(),
            format!("{} {} $module_pattern", module_field, mod_match),
            "c.callee_function <> '%'".to_string(),
            // Join Call with FunctionLocation on caller
            // caller_function in Call includes arity like "render/2", so use STARTS WITH
            "loc.module = c.caller_module".to_string(),
            "c.caller_function STARTS WITH loc.name".to_string(),
            "c.line >= loc.start_line".to_string(),
            "c.line <= loc.end_line".to_string(),
        ];

        // Add function filter if present
        if self.function_pattern.is_some() {
            where_conditions.push(format!("{} {} $function_pattern", function_field, fn_match));
        }

        // Add arity filter if present
        if self.arity.is_some() {
            where_conditions.push(format!("{} = $arity", arity_field));
        }

        let where_clause = where_conditions.join("\n  AND ");

        // Order clause depends on direction
        let order_clause = match self.direction {
            CallDirection::From => {
                "c.caller_module, loc.name, loc.arity, c.line"
            }
            CallDirection::To => {
                "c.callee_module, c.callee_function, c.callee_arity, c.caller_module, loc.name"
            }
        };

        Ok(format!(
            r#"MATCH (c:Call), (loc:FunctionLocation)
WHERE {where_clause}
RETURN c.caller_module, loc.name AS caller_name, loc.arity AS caller_arity,
       loc.kind AS caller_kind, loc.start_line AS caller_start_line, loc.end_line AS caller_end_line,
       c.callee_module, c.callee_function, c.callee_arity,
       c.file, c.line AS call_line, c.call_type
ORDER BY {order_clause}
LIMIT {}"#,
            self.limit
        ))
    }
}

/// Find calls in the specified direction.
///
/// - `From`: Returns all calls made by functions matching the pattern
/// - `To`: Returns all calls to functions matching the pattern
pub fn find_calls(
    db: &dyn DatabaseBackend,
    direction: CallDirection,
    module_pattern: &str,
    function_pattern: Option<&str>,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    let builder = CallsQueryBuilder {
        direction,
        module_pattern: module_pattern.to_string(),
        function_pattern: function_pattern.map(|s| s.to_string()),
        arity,
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| CallsError::QueryFailed {
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
    fn test_calls_query_cozo_from_direction() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::From,
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: None,
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("*calls"));
        assert!(compiled.contains("*function_locations"));
        assert!(compiled.contains("caller_module"));
        assert!(compiled.contains("callee_function != '%'"));
    }

    #[test]
    fn test_calls_query_cozo_to_direction() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::To,
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: Some("handle_call".to_string()),
            arity: Some(3),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("callee_module"));
        assert!(compiled.contains("callee_function"));
        assert!(compiled.contains("callee_arity"));
    }

    #[test]
    fn test_calls_query_age_from_direction() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::From,
            module_pattern: "MyApp".to_string(),
            function_pattern: None,
            arity: None,
            project: "myproject".to_string(),
            use_regex: true,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (c:Call), (loc:FunctionLocation)"));
        assert!(compiled.contains("c.caller_module =~"));
        assert!(compiled.contains("c.callee_function <> '%'"));
    }

    #[test]
    fn test_calls_query_parameters() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::From,
            module_pattern: "mod".to_string(),
            function_pattern: Some("func".to_string()),
            arity: Some(2),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 4);
        assert!(params.contains_key("module_pattern"));
        assert!(params.contains_key("function_pattern"));
        assert!(params.contains_key("arity"));
        assert!(params.contains_key("project"));
    }

    #[test]
    fn test_calls_query_cozo_with_regex() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::From,
            module_pattern: "MyApp.*".to_string(),
            function_pattern: None,
            arity: None,
            project: "myproject".to_string(),
            use_regex: true,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_calls_query_cozo_with_function_pattern() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::From,
            module_pattern: "mod".to_string(),
            function_pattern: Some("test_func".to_string()),
            arity: None,
            project: "proj".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("caller_name == $function_pattern"));
    }

    #[test]
    fn test_calls_query_cozo_with_arity() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::From,
            module_pattern: "mod".to_string(),
            function_pattern: None,
            arity: Some(2),
            project: "proj".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("caller_arity == $arity"));
    }

    #[test]
    fn test_calls_query_age_to_direction() {
        let builder = CallsQueryBuilder {
            direction: CallDirection::To,
            module_pattern: "MyApp".to_string(),
            function_pattern: Some("handle".to_string()),
            arity: Some(2),
            project: "myproject".to_string(),
            use_regex: false,
            limit: 50,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching for "To" direction
        assert!(compiled.contains("MATCH (c:Call), (loc:FunctionLocation)"));
        assert!(compiled.contains("c.callee_function = $function_pattern"));
        assert!(compiled.contains("c.callee_arity = $arity"));
    }

    #[test]
    fn test_calls_query_direction_enum() {
        assert_eq!(CallDirection::From.filter_fields(), ("caller_module", "caller_name", "caller_arity"));
        assert_eq!(CallDirection::To.filter_fields(), ("callee_module", "callee_function", "callee_arity"));
    }
}
