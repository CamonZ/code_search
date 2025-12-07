use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::{SearchCmd, SearchKind};
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, extract_string_or, open_db, run_query, Params};

#[derive(Error, Debug)]
enum SearchError {
    #[error("Search failed: {message}")]
    QueryFailed { message: String },
}

/// A module search result
#[derive(Debug, Clone, Serialize)]
pub struct ModuleResult {
    pub project: String,
    pub name: String,
    pub source: String,
}

/// A function search result
#[derive(Debug, Clone, Serialize)]
pub struct FunctionResult {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub return_type: String,
}

/// Result of the search command execution
#[derive(Debug, Default, Serialize)]
pub struct SearchResult {
    pub pattern: String,
    pub kind: String,
    pub modules: Vec<ModuleResult>,
    pub functions: Vec<FunctionResult>,
}

impl Execute for SearchCmd {
    type Output = SearchResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = SearchResult {
            pattern: self.pattern.clone(),
            kind: match self.kind {
                SearchKind::Modules => "modules".to_string(),
                SearchKind::Functions => "functions".to_string(),
            },
            ..Default::default()
        };

        match self.kind {
            SearchKind::Modules => {
                result.modules = search_modules(&db, &self.pattern, &self.project, self.limit, self.regex)?;
            }
            SearchKind::Functions => {
                result.functions = search_functions(&db, &self.pattern, &self.project, self.limit, self.regex)?;
            }
        }

        Ok(result)
    }
}

fn search_modules(
    db: &cozo::DbInstance,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<ModuleResult>, Box<dyn Error>> {
    let match_fn = if use_regex { "regex_matches" } else { "str_includes" };
    let script = format!(
        r#"
        ?[project, name, source] := *modules{{project, name, source}},
            project = $project,
            {match_fn}(name, $pattern)
        :limit {limit}
        :order name
        "#,
    );

    let mut params = Params::new();
    params.insert("pattern".to_string(), DataValue::Str(pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 3 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(name) = extract_string(&row[1]) else { continue };
            let source = extract_string_or(&row[2], "unknown");
            results.push(ModuleResult { project, name, source });
        }
    }

    Ok(results)
}

fn search_functions(
    db: &cozo::DbInstance,
    pattern: &str,
    project: &str,
    limit: u32,
    use_regex: bool,
) -> Result<Vec<FunctionResult>, Box<dyn Error>> {
    let match_fn = if use_regex { "regex_matches" } else { "str_includes" };
    let script = format!(
        r#"
        ?[project, module, name, arity, return_type] := *functions{{project, module, name, arity, return_type}},
            project = $project,
            {match_fn}(name, $pattern)
        :limit {limit}
        :order module, name, arity
        "#,
    );

    let mut params = Params::new();
    params.insert("pattern".to_string(), DataValue::Str(pattern.into()));
    params.insert("project".to_string(), DataValue::Str(project.into()));

    let rows = run_query(db, &script, params).map_err(|e| SearchError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 5 {
            let Some(project) = extract_string(&row[0]) else { continue };
            let Some(module) = extract_string(&row[1]) else { continue };
            let Some(name) = extract_string(&row[2]) else { continue };
            let arity = extract_i64(&row[3], 0);
            let return_type = extract_string_or(&row[4], "");
            results.push(FunctionResult {
                project,
                module,
                name,
                arity,
                return_type,
            });
        }
    }

    Ok(results)
}
