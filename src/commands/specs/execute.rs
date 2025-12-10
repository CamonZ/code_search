use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::SpecsCmd;
use crate::commands::Execute;
use crate::queries::specs::{find_specs, SpecDef};

/// A single spec definition
#[derive(Debug, Clone, Serialize)]
pub struct SpecEntry {
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub inputs: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub returns: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub full: String,
}

/// A module containing specs
#[derive(Debug, Clone, Serialize)]
pub struct SpecModule {
    pub name: String,
    pub specs: Vec<SpecEntry>,
}

/// Result of the specs command execution
#[derive(Debug, Default, Serialize)]
pub struct SpecsResult {
    pub module_pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_filter: Option<String>,
    pub total_specs: usize,
    pub modules: Vec<SpecModule>,
}

impl SpecsResult {
    /// Build grouped result from flat SpecDef list
    fn from_specs(
        module_pattern: String,
        function_pattern: Option<String>,
        kind_filter: Option<String>,
        specs: Vec<SpecDef>,
    ) -> Self {
        let total_specs = specs.len();

        // Group by module (BTreeMap for consistent ordering)
        let mut module_map: BTreeMap<String, Vec<SpecEntry>> = BTreeMap::new();

        for spec in specs {
            let entry = SpecEntry {
                name: spec.name,
                arity: spec.arity,
                kind: spec.kind,
                line: spec.line,
                inputs: spec.inputs_string,
                returns: spec.return_string,
                full: spec.full,
            };

            module_map.entry(spec.module).or_default().push(entry);
        }

        let modules: Vec<SpecModule> = module_map
            .into_iter()
            .map(|(name, specs)| SpecModule { name, specs })
            .collect();

        SpecsResult {
            module_pattern,
            function_pattern,
            kind_filter,
            total_specs,
            modules,
        }
    }
}

impl Execute for SpecsCmd {
    type Output = SpecsResult;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let specs = find_specs(
            db,
            &self.module,
            self.function.as_deref(),
            self.kind.as_deref(),
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(SpecsResult::from_specs(
            self.module,
            self.function,
            self.kind,
            specs,
        ))
    }
}
