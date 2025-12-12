use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::FunctionCmd;
use crate::commands::Execute;
use crate::queries::function::{find_functions, FunctionSignature};
use crate::types::{ModuleGroupResult, ModuleGroup};

/// A function signature within a module
#[derive(Debug, Clone, Serialize)]
pub struct FuncSig {
    pub name: String,
    pub arity: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub args: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub return_type: String,
}

impl ModuleGroupResult<FuncSig> {
    /// Build grouped result from flat FunctionSignature list
    fn from_signatures(
        module_pattern: String,
        function_pattern: String,
        signatures: Vec<FunctionSignature>,
    ) -> Self {
        let total_items = signatures.len();

        // Group by module (BTreeMap for consistent ordering)
        let mut module_map: BTreeMap<String, Vec<FuncSig>> = BTreeMap::new();

        for sig in signatures {
            let func_sig = FuncSig {
                name: sig.name,
                arity: sig.arity,
                args: sig.args,
                return_type: sig.return_type,
            };

            module_map.entry(sig.module).or_default().push(func_sig);
        }

        let items: Vec<ModuleGroup<FuncSig>> = module_map
            .into_iter()
            .map(|(name, entries)| ModuleGroup {
                name,
                // File is intentionally empty for functions because the function command
                // queries the functions table which doesn't track file locations.
                // File locations are available in function_locations table if needed.
                file: String::new(),
                entries,
            })
            .collect();

        ModuleGroupResult {
            module_pattern,
            function_pattern: Some(function_pattern),
            total_items,
            items,
        }
    }
}

impl Execute for FunctionCmd {
    type Output = ModuleGroupResult<FuncSig>;

    fn execute(self, db: &cozo::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let signatures = find_functions(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(<ModuleGroupResult<FuncSig>>::from_signatures(
            self.module,
            self.function,
            signatures,
        ))
    }
}
