use crate::db::DatabaseBackend;
use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum ReverseTraceError {
    #[error("Reverse trace query failed: {message}")]
    QueryFailed { message: String },
}

/// Query builder for recursive reverse call tracing (finding callers of a target)
#[derive(Debug)]
pub struct ReverseTraceQueryBuilder {
    pub module_pattern: String,
    pub function_pattern: String,
    pub arity: Option<i64>,
    pub project: String,
    pub use_regex: bool,
    pub max_depth: u32,
    pub limit: u32,
}

impl QueryBuilder for ReverseTraceQueryBuilder {
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
        params.insert("function_pattern".to_string(), DataValue::Str(self.function_pattern.clone().into()));
        if let Some(a) = self.arity {
            params.insert("arity".to_string(), DataValue::from(a));
        }
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        params
    }
}

impl ReverseTraceQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build conditions for the base case
        let module_cond = crate::utils::ConditionBuilder::new("callee_module", "module_pattern").build(self.use_regex);
        let function_cond = crate::utils::ConditionBuilder::new("callee_function", "function_pattern").build(self.use_regex);
        let arity_cond = crate::utils::OptionalConditionBuilder::new("callee_arity", "arity")
            .when_none("true")
            .build(self.arity.is_some());

        Ok(format!(
            r#"
        # Base case: calls TO the target function, joined with function_locations
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            {module_cond},
            {function_cond},
            project == $project,
            {arity_cond},
            depth = 1

        # Recursive case: calls TO the callers we've found
        # Note: prev_caller_function has arity suffix (e.g., "foo/2") but callee_function doesn't (e.g., "foo")
        # So we use starts_with to match prev_caller_function starting with callee_function
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            trace[prev_depth, prev_caller_module, prev_caller_name, prev_caller_arity, _, _, _, _, _, _, _, _],
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            callee_module == prev_caller_module,
            callee_function == prev_caller_name,
            callee_arity == prev_caller_arity,
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            prev_depth < {max_depth},
            depth = prev_depth + 1,
            project == $project

        ?[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line]

        :order depth, caller_module, caller_name, caller_arity, call_line, callee_module, callee_function, callee_arity
        :limit {limit}
        "#,
            max_depth = self.max_depth,
            limit = self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };
        let fn_match = if self.use_regex { "=~" } else { "=" };

        // Build WHERE conditions for the target function
        let mut where_conditions = vec![
            "target.project = $project".to_string(),
            format!("target.module {} $module_pattern", mod_match),
            format!("target.name {} $function_pattern", fn_match),
        ];

        // Add arity filter if present
        if self.arity.is_some() {
            where_conditions.push("target.arity = $arity".to_string());
        }

        let where_clause = where_conditions.join("\n  AND ");

        Ok(format!(
            r#"MATCH path = (caller:Function)-[:CALLS*1..{max_depth}]->(target:Function)
WHERE {where_clause}
WITH path, length(path) as depth,
     nodes(path) as funcs,
     relationships(path) as calls
UNWIND range(0, size(calls)-1) as idx
WITH depth,
     funcs[idx] as caller,
     funcs[idx+1] as callee,
     calls[idx] as call
MATCH (caller)-[:DEFINED_IN]->(loc:FunctionLocation)
RETURN depth,
       caller.module, caller.name, caller.arity, loc.kind,
       loc.start_line, loc.end_line,
       callee.module, callee.name, callee.arity,
       call.file, call.line
ORDER BY depth, caller.module, caller.name, call.line
LIMIT {limit}"#,
            max_depth = self.max_depth,
            limit = self.limit,
            where_clause = where_clause
        ))
    }
}

/// A single step in the reverse call chain
#[derive(Debug, Clone, Serialize)]
pub struct ReverseTraceStep {
    pub depth: i64,
    pub caller_module: String,
    pub caller_function: String,
    pub caller_arity: i64,
    pub caller_kind: String,
    pub caller_start_line: i64,
    pub caller_end_line: i64,
    pub callee_module: String,
    pub callee_function: String,
    pub callee_arity: i64,
    pub file: String,
    pub line: i64,
}

pub fn reverse_trace_calls(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<ReverseTraceStep>, Box<dyn Error>> {
    let builder = ReverseTraceQueryBuilder {
        module_pattern: module_pattern.to_string(),
        function_pattern: function_pattern.to_string(),
        arity,
        project: project.to_string(),
        use_regex,
        max_depth,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| ReverseTraceError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 12 {
            let depth = extract_i64(&row[0], 0);
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_function) = extract_string(&row[2]) else { continue };
            let caller_arity = extract_i64(&row[3], 0);
            let caller_kind = extract_string_or(&row[4], "");
            let caller_start_line = extract_i64(&row[5], 0);
            let caller_end_line = extract_i64(&row[6], 0);
            let Some(callee_module) = extract_string(&row[7]) else { continue };
            let Some(callee_function) = extract_string(&row[8]) else { continue };
            let callee_arity = extract_i64(&row[9], 0);
            let Some(file) = extract_string(&row[10]) else { continue };
            let line = extract_i64(&row[11], 0);

            results.push(ReverseTraceStep {
                depth,
                caller_module,
                caller_function,
                caller_arity,
                caller_kind,
                caller_start_line,
                caller_end_line,
                callee_module,
                callee_function,
                callee_arity,
                file,
                line,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_reverse_trace_query_cozo_basic() {
        let builder = ReverseTraceQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: "handle_call".to_string(),
            arity: Some(3),
            project: "myproject".to_string(),
            use_regex: false,
            max_depth: 5,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Verify recursive structure
        assert!(compiled.contains("trace[depth"));
        assert!(compiled.contains("depth = 1"));
        assert!(compiled.contains("depth = prev_depth + 1"));
        assert!(compiled.contains("callee_module == prev_caller_module"));
    }

    #[test]
    fn test_reverse_trace_query_cozo_regex() {
        let builder = ReverseTraceQueryBuilder {
            module_pattern: "MyApp\\..*".to_string(),
            function_pattern: "handle_.*".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: true,
            max_depth: 10,
            limit: 500,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches(callee_module"));
        assert!(compiled.contains("regex_matches(callee_function"));
    }

    #[test]
    fn test_reverse_trace_query_age() {
        let builder = ReverseTraceQueryBuilder {
            module_pattern: "MyApp".to_string(),
            function_pattern: "target".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            max_depth: 5,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH"));
        assert!(compiled.contains("CALLS*1..5"));
        assert!(compiled.contains("target.module")); // Filter on target
    }

    #[test]
    fn test_reverse_trace_query_parameters() {
        let builder = ReverseTraceQueryBuilder {
            module_pattern: "mod".to_string(),
            function_pattern: "func".to_string(),
            arity: Some(2),
            project: "proj".to_string(),
            use_regex: false,
            max_depth: 3,
            limit: 50,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 4);
        assert!(params.contains_key("module_pattern"));
        assert!(params.contains_key("function_pattern"));
        assert!(params.contains_key("arity"));
        assert!(params.contains_key("project"));
    }
}
