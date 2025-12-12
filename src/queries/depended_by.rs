use std::error::Error;

use cozo::DataValue;
use thiserror::Error;

use crate::db::{extract_call_from_row, run_query, CallRowLayout, Params};
use crate::types::Call;

#[derive(Error, Debug)]
pub enum DependedByError {
    #[error("Dependency query failed: {message}")]
    QueryFailed { message: String },
}

/// Find all calls from external modules to the target module, with function-level detail
pub fn find_dependents(
    db: &cozo::DbInstance,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Call>, Box<dyn Error>> {
    // Build module condition using helper
    let module_cond = crate::utils::ConditionBuilder::new("callee_module", "module_pattern").build(use_regex);

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

    let layout = CallRowLayout::without_extras();
    let results = rows.rows.iter()
        .filter_map(|row| {
            if row.len() >= 11 {
                extract_call_from_row(row, &layout)
            } else {
                None
            }
        })
        .collect();

    Ok(results)
}
