use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::LocationCmd;
use crate::commands::Execute;
use crate::queries::location::{find_locations, FunctionLocation};

/// A single clause (definition) of a function
#[derive(Debug, Clone, Serialize)]
pub struct LocationClause {
    pub line: i64,
    pub start_line: i64,
    pub end_line: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pattern: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub guard: String,
}

/// A function with all its clauses grouped together
#[derive(Debug, Clone, Serialize)]
pub struct LocationFunction {
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub file: String,
    pub clauses: Vec<LocationClause>,
}

/// A module containing functions
#[derive(Debug, Clone, Serialize)]
pub struct LocationModule {
    pub name: String,
    pub functions: Vec<LocationFunction>,
}

/// Result of the location command execution
#[derive(Debug, Default, Serialize)]
pub struct LocationResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub total_clauses: usize,
    pub modules: Vec<LocationModule>,
}

impl LocationResult {
    /// Build grouped result from flat FunctionLocation list
    fn from_locations(
        module_pattern: String,
        function_pattern: String,
        locations: Vec<FunctionLocation>,
    ) -> Self {
        let total_clauses = locations.len();

        // Group by module, then by (function_name, arity)
        // Use BTreeMap for consistent ordering
        let mut module_map: BTreeMap<String, BTreeMap<(String, i64), (String, String, Vec<LocationClause>)>> =
            BTreeMap::new();

        for loc in locations {
            let func_key = (loc.name.clone(), loc.arity);
            let clause = LocationClause {
                line: loc.line,
                start_line: loc.start_line,
                end_line: loc.end_line,
                pattern: loc.pattern,
                guard: loc.guard,
            };

            module_map
                .entry(loc.module.clone())
                .or_default()
                .entry(func_key)
                .or_insert_with(|| (loc.kind.clone(), loc.file.clone(), Vec::new()))
                .2
                .push(clause);
        }

        // Convert to final structure
        let modules: Vec<LocationModule> = module_map
            .into_iter()
            .map(|(module_name, funcs)| {
                let functions: Vec<LocationFunction> = funcs
                    .into_iter()
                    .map(|((name, arity), (kind, file, clauses))| LocationFunction {
                        name,
                        arity,
                        kind,
                        file,
                        clauses,
                    })
                    .collect();
                LocationModule {
                    name: module_name,
                    functions,
                }
            })
            .collect();

        LocationResult {
            module_pattern,
            function_pattern,
            total_clauses,
            modules,
        }
    }
}

impl Execute for LocationCmd {
    type Output = LocationResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let locations = find_locations(
            db,
            self.module.as_deref(),
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(LocationResult::from_locations(
            self.module.unwrap_or_default(),
            self.function,
            locations,
        ))
    }
}
