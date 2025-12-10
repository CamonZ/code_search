use std::error::Error;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_i64, extract_string, extract_string_or, run_query, Params};

#[derive(Error, Debug)]
pub enum DependedByError {
    #[error("Dependency query failed: {message}")]
    QueryFailed { message: String },
}

/// A call from a dependent module to the target
#[derive(Debug, Clone, Serialize)]
pub struct DependentCall {
    pub caller_module: String,
    pub caller_function: String,
    pub caller_arity: i64,
    pub caller_kind: String,
    pub caller_start_line: i64,
    pub caller_end_line: i64,
    pub callee_function: String,
    pub callee_arity: i64,
    pub file: String,
    pub line: i64,
}

/// Find all calls from external modules to the target module, with function-level detail
pub fn find_dependents(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<DependentCall>, Box<dyn Error>> {
    let module_cond = if use_regex {
        "regex_matches(callee_module, $module_pattern)"
    } else {
        "callee_module == $module_pattern"
    };

    // Query calls with function_locations join for caller metadata, excluding self-references
    let script = format!(
        r#"
        ?[caller_module, caller_name, caller_arity, caller_kind, caller_start_line, caller_end_line, callee_function, callee_arity, file, call_line] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line: call_line}},
            *function_locations{{project, module: caller_module, name: caller_name, arity: caller_arity, kind: caller_kind, start_line: caller_start_line, end_line: caller_end_line}},
            starts_with(caller_function, caller_name),
            call_line >= caller_start_line,
            call_line <= caller_end_line,
            {module_cond},
            caller_module != callee_module,
            project == $project
        :order caller_module, caller_name, caller_arity, callee_function, callee_arity, call_line
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("module_pattern".to_string(), DataValue::Str(module_pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| DependedByError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 10 {
            let Some(caller_module) = extract_string(&row[0]) else { continue };
            let Some(caller_function) = extract_string(&row[1]) else { continue };
            let caller_arity = extract_i64(&row[2], 0);
            let caller_kind = extract_string_or(&row[3], "");
            let caller_start_line = extract_i64(&row[4], 0);
            let caller_end_line = extract_i64(&row[5], 0);
            let Some(callee_function) = extract_string(&row[6]) else { continue };
            let callee_arity = extract_i64(&row[7], 0);
            let Some(file) = extract_string(&row[8]) else { continue };
            let line = extract_i64(&row[9], 0);

            results.push(DependentCall {
                caller_module,
                caller_function,
                caller_arity,
                caller_kind,
                caller_start_line,
                caller_end_line,
                callee_function,
                callee_arity,
                file,
                line,
            });
        }
    }

    Ok(results)
}
