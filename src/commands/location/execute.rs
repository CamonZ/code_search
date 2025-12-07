use std::error::Error;
use std::path::Path;

use cozo::{DataValue, Num};
use serde::Serialize;
use thiserror::Error;

use super::LocationCmd;
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, extract_string_or, open_db, run_query, Params};

#[derive(Error, Debug)]
enum LocationError {
    #[error("Location query failed: {message}")]
    QueryFailed { message: String },
}

/// A function location result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionLocation {
    pub project: String,
    pub file: String,
    pub start_line: i64,
    pub end_line: i64,
    pub module: String,
    pub kind: String,
    pub name: String,
    pub arity: i64,
}

impl FunctionLocation {
    /// Format as file:start_line:end_line
    pub fn format_location(&self) -> String {
        format!("{}:{}:{}", self.file, self.start_line, self.end_line)
    }
}

/// Result of the location command execution
#[derive(Debug, Default, Serialize)]
pub struct LocationResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub locations: Vec<FunctionLocation>,
}

impl Execute for LocationCmd {
    type Output = LocationResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = LocationResult {
            module_pattern: self.module.clone().unwrap_or_default(),
            function_pattern: self.function.clone(),
            ..Default::default()
        };

        result.locations = find_locations(
            &db,
            self.module.as_deref(),
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_locations(
    db: &cozo::DbInstance,
    module_pattern: Option<&str>,
    function_pattern: &str,
    arity: Option<i64>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<FunctionLocation>, Box<dyn Error>> {
    // Build the query based on whether we're using regex or exact match
    let fn_cond = if use_regex {
        "regex_matches(name, $function_pattern)".to_string()
    } else {
        "name == $function_pattern".to_string()
    };

    let module_cond = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", module == $module_pattern".to_string(),
        None => String::new(),
    };

    let arity_cond = if arity.is_some() {
        ", arity == $arity"
    } else {
        ""
    };

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, file, start_line, end_line, module, kind, name, arity] :=
            *function_locations{{project, module, name, arity, file, kind, start_line, end_line}},
            {fn_cond}
            {module_cond}
            {arity_cond}
            {project_cond}
        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("function_pattern".to_string(), DataValue::Str(function_pattern.into()));
    if let Some(mod_pat) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(mod_pat.into()));
    }
    if let Some(a) = arity {
        params.insert("arity".to_string(), DataValue::Num(Num::Int(a)));
    }
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| LocationError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 8 {
            // Order matches query: project, file, start_line, end_line, module, kind, name, arity
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(file) = extract_string(&row[1]) else { continue };
            let start_line = extract_i64(&row[2], 0);
            let end_line = extract_i64(&row[3], 0);
            let Some(module) = extract_string(&row[4]) else { continue };
            let kind = extract_string_or(&row[5], "");
            let Some(name) = extract_string(&row[6]) else { continue };
            let arity = extract_i64(&row[7], 0);

            results.push(FunctionLocation {
                project,
                file,
                start_line,
                end_line,
                module,
                kind,
                name,
                arity,
            });
        }
    }

    Ok(results)
}
