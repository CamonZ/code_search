use std::error::Error;

use serde::Serialize;

use super::FunctionCmd;
use crate::commands::Execute;
use db::queries::function::{find_functions, FunctionSignature};
use db::types::ModuleGroupResult;

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

/// Build grouped result from flat FunctionSignature list
fn build_function_signatures_result(
    module_pattern: String,
    function_pattern: String,
    signatures: Vec<FunctionSignature>,
) -> ModuleGroupResult<FuncSig> {
    let total_items = signatures.len();

    // Use helper to group by module
    let items = crate::utils::group_by_module(signatures, |sig| {
        let func_sig = FuncSig {
            name: sig.name,
            arity: sig.arity,
            args: sig.args,
            return_type: sig.return_type,
        };
        // File is intentionally empty for functions because the function command
        // queries the functions table which doesn't track file locations.
        // File locations are available in function_locations table if needed.
        (sig.module, func_sig)
    });

    ModuleGroupResult {
        module_pattern,
        function_pattern: Some(function_pattern),
        total_items,
        items,
    }
}

impl Execute for FunctionCmd {
    type Output = ModuleGroupResult<FuncSig>;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let signatures = find_functions(
            db,
            &self.module,
            &self.function,
            self.arity,
            &self.common.project,
            self.common.regex,
            self.common.limit,
        )?;

        Ok(build_function_signatures_result(
            self.module,
            self.function,
            signatures,
        ))
    }
}
