use std::error::Error;

use serde::Serialize;

use super::{SearchCmd, SearchKind};
use crate::commands::Execute;
use crate::queries::search::{search_functions, search_modules, FunctionResult, ModuleResult};

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

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
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
                result.modules = search_modules(db, &self.pattern, &self.project, self.limit, self.regex)?;
            }
            SearchKind::Functions => {
                result.functions = search_functions(db, &self.pattern, &self.project, self.limit, self.regex)?;
            }
        }

        Ok(result)
    }
}