use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::{SearchCmd, SearchKind};
use crate::commands::Execute;
use crate::queries::search::{search_functions, search_modules, FunctionResult as RawFunctionResult, ModuleResult};

/// A function found in search results
#[derive(Debug, Clone, Serialize)]
pub struct SearchFunc {
    pub name: String,
    pub arity: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub return_type: String,
}

/// A module containing functions in search results
#[derive(Debug, Clone, Serialize)]
pub struct SearchFuncModule {
    pub name: String,
    pub functions: Vec<SearchFunc>,
}

/// Result of the search command execution
#[derive(Debug, Default, Serialize)]
pub struct SearchResult {
    pub pattern: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<ModuleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_functions: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub function_modules: Vec<SearchFuncModule>,
}

impl SearchResult {
    /// Build grouped function result from flat list
    fn from_functions(pattern: String, functions: Vec<RawFunctionResult>) -> Self {
        let total = functions.len();

        // Group by module (BTreeMap for consistent ordering)
        let mut module_map: BTreeMap<String, Vec<SearchFunc>> = BTreeMap::new();

        for func in functions {
            let search_func = SearchFunc {
                name: func.name,
                arity: func.arity,
                return_type: func.return_type,
            };

            module_map.entry(func.module).or_default().push(search_func);
        }

        let function_modules: Vec<SearchFuncModule> = module_map
            .into_iter()
            .map(|(name, functions)| SearchFuncModule { name, functions })
            .collect();

        SearchResult {
            pattern,
            kind: "functions".to_string(),
            modules: vec![],
            total_functions: if total > 0 { Some(total) } else { None },
            function_modules,
        }
    }
}

impl Execute for SearchCmd {
    type Output = SearchResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        match self.kind {
            SearchKind::Modules => {
                let modules = search_modules(db, &self.pattern, &self.common.project, self.common.limit, self.common.regex)?;
                Ok(SearchResult {
                    pattern: self.pattern,
                    kind: "modules".to_string(),
                    modules,
                    total_functions: None,
                    function_modules: vec![],
                })
            }
            SearchKind::Functions => {
                let functions = search_functions(db, &self.pattern, &self.common.project, self.common.limit, self.common.regex)?;
                Ok(SearchResult::from_functions(self.pattern, functions))
            }
        }
    }
}