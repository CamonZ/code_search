use std::collections::BTreeMap;
use std::error::Error;

use serde::Serialize;

use super::FunctionCmd;
use crate::commands::Execute;
use crate::queries::function::{find_functions, FunctionSignature};

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

/// A module containing function signatures
#[derive(Debug, Clone, Serialize)]
pub struct FuncModule {
    pub name: String,
    pub functions: Vec<FuncSig>,
}

/// Result of the function command execution
#[derive(Debug, Default, Serialize)]
pub struct FunctionResult {
    pub module_pattern: String,
    pub function_pattern: String,
    pub total_functions: usize,
    pub modules: Vec<FuncModule>,
}

impl FunctionResult {
    /// Build grouped result from flat FunctionSignature list
    fn from_signatures(
        module_pattern: String,
        function_pattern: String,
        signatures: Vec<FunctionSignature>,
    ) -> Self {
        let total_functions = signatures.len();

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

        let modules: Vec<FuncModule> = module_map
            .into_iter()
            .map(|(name, functions)| FuncModule { name, functions })
            .collect();

        FunctionResult {
            module_pattern,
            function_pattern,
            total_functions,
            modules,
        }
    }
}

impl Execute for FunctionCmd {
    type Output = FunctionResult;

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

        Ok(FunctionResult::from_signatures(
            self.module,
            self.function,
            signatures,
        ))
    }
}
