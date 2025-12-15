use crate::db::DatabaseBackend;
use std::error::Error;
use std::rc::Rc;

use cozo::DataValue;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};
use crate::types::{Call, FunctionRef};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

#[derive(Error, Debug)]
pub enum TraceError {
    #[error("Trace query failed: {message}")]
    QueryFailed { message: String },
}

/// Query builder for recursive call tracing
#[derive(Debug)]
pub struct TraceQueryBuilder {
    pub module_pattern: String,
    pub function_pattern: String,
    pub arity: Option<i64>,
    pub project: String,
    pub use_regex: bool,
    pub max_depth: u32,
    pub limit: u32,
}

impl QueryBuilder for TraceQueryBuilder {
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

impl TraceQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        // Build conditions for the base case
        let module_cond = crate::utils::ConditionBuilder::new("caller_module", "module_pattern").build(self.use_regex);
        let function_cond = crate::utils::ConditionBuilder::new("caller_name", "function_pattern").build(self.use_regex);
        let arity_cond = crate::utils::OptionalConditionBuilder::new("callee_arity", "arity")
            .when_none("true")
            .build(self.arity.is_some());

        Ok(format!(
            r#"
        # Base case: calls from the starting function, joined with function_locations
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            callee_function != '%',
            {module_cond},
            {function_cond},
            project == $project,
            {arity_cond},
            depth = 1

        # Recursive case: calls from callees we've found
        trace[depth, caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_module, callee_function, callee_arity, file, call_line] :=
            trace[prev_depth, _, _, _, _, _, _, prev_callee_module, prev_callee_function, _, _, _],
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            caller_module == prev_callee_module,
            starts_with(caller_function, caller_name),
            starts_with(caller_function, prev_callee_function),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            callee_function != '%',
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
        // AGE data model uses vertices only, not edges.
        // Variable-length path traversal requires recursive execution.
        // This simplified version returns all calls, and filtering/depth computation
        // should be handled by the caller if recursive traversal is needed.
        //
        // Note: For proper recursive trace, use the Cozo backend or implement
        // iterative query execution in Rust.

        let mod_match = if self.use_regex { "=~" } else { "=" };
        let fn_match = if self.use_regex { "=~" } else { "=" };

        // Build WHERE conditions for the starting function
        let mut where_conditions = vec![
            "c.project = $project".to_string(),
            format!("c.caller_module {} $module_pattern", mod_match),
            format!("loc.name {} $function_pattern", fn_match),
            "c.callee_function <> '%'".to_string(),
            "loc.module = c.caller_module".to_string(),
            "c.caller_function STARTS WITH loc.name".to_string(),
            "c.line >= loc.start_line".to_string(),
            "c.line <= loc.end_line".to_string(),
        ];

        // Add arity filter if present
        if self.arity.is_some() {
            where_conditions.push("loc.arity = $arity".to_string());
        }

        let where_clause = where_conditions.join("\n  AND ");

        Ok(format!(
            r#"MATCH (c:Call), (loc:FunctionLocation)
WHERE {where_clause}
RETURN 1 AS depth,
       c.caller_module, loc.name AS caller_name, loc.arity AS caller_arity, loc.kind AS caller_kind,
       loc.start_line AS caller_start_line, loc.end_line AS caller_end_line,
       c.callee_module, c.callee_function, c.callee_arity,
       c.file, c.line AS call_line
ORDER BY c.caller_module, loc.name, c.line
LIMIT {limit}"#,
            limit = self.limit,
            where_clause = where_clause
        ))
    }
}

pub fn trace_calls(
    db: &dyn DatabaseBackend,
    module_pattern: &str,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    max_depth: u32,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    let builder = TraceQueryBuilder {
        module_pattern: module_pattern.to_string(),
        function_pattern: function_pattern.to_string(),
        arity,
        project: project.to_string(),
        use_regex,
        max_depth,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| TraceError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 12 {
            let depth = extract_i64(&row[0], 0);
            let Some(caller_module) = extract_string(&row[1]) else { continue };
            let Some(caller_name) = extract_string(&row[2]) else { continue };
            let caller_arity = extract_i64(&row[3], 0);
            let caller_kind = extract_string_or(&row[4], "");
            let caller_start_line = extract_i64(&row[5], 0);
            let caller_end_line = extract_i64(&row[6], 0);
            let Some(callee_module) = extract_string(&row[7]) else { continue };
            let Some(callee_name) = extract_string(&row[8]) else { continue };
            let callee_arity = extract_i64(&row[9], 0);
            let Some(file) = extract_string(&row[10]) else { continue };
            let line = extract_i64(&row[11], 0);

            let caller = FunctionRef::with_definition(
                Rc::from(caller_module.into_boxed_str()),
                Rc::from(caller_name.into_boxed_str()),
                caller_arity,
                Rc::from(caller_kind.into_boxed_str()),
                Rc::from(file.into_boxed_str()),
                caller_start_line,
                caller_end_line,
            );

            // Callee doesn't have definition info from this query
            let callee = FunctionRef::new(
                Rc::from(callee_module.into_boxed_str()),
                Rc::from(callee_name.into_boxed_str()),
                callee_arity,
            );

            results.push(Call {
                caller,
                callee,
                line,
                call_type: None,
                depth: Some(depth),
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
    fn test_trace_query_cozo_basic() {
        let builder = TraceQueryBuilder {
            module_pattern: "MyApp.Server".to_string(),
            function_pattern: "start".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            max_depth: 5,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        // Verify recursive structure
        assert!(compiled.contains("trace[depth"));
        assert!(compiled.contains("depth = 1")); // Base case
        assert!(compiled.contains("depth = prev_depth + 1")); // Recursive case
        assert!(compiled.contains("prev_depth < 5")); // Max depth check
    }

    #[test]
    fn test_trace_query_cozo_with_arity() {
        let builder = TraceQueryBuilder {
            module_pattern: "MyApp".to_string(),
            function_pattern: "init".to_string(),
            arity: Some(1),
            project: "myproject".to_string(),
            use_regex: true,
            max_depth: 10,
            limit: 500,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
        assert!(compiled.contains("callee_arity == $arity") || compiled.contains("$arity"));
    }

    #[test]
    fn test_trace_query_age() {
        let builder = TraceQueryBuilder {
            module_pattern: "MyApp".to_string(),
            function_pattern: "start".to_string(),
            arity: None,
            project: "myproject".to_string(),
            use_regex: false,
            max_depth: 5,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (c:Call), (loc:FunctionLocation)"));
        assert!(compiled.contains("c.caller_module = $module_pattern"));
        assert!(compiled.contains("loc.name = $function_pattern"));
    }

    #[test]
    fn test_trace_query_parameters() {
        let builder = TraceQueryBuilder {
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
