//! JSON import structures for call graph data.
//!
//! These types are used to deserialize the JSON output from the Elixir
//! call graph extractor during the import process.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CallGraph {
    // Objects
    pub structs: HashMap<String, StructDef>,
    pub function_locations: HashMap<String, HashMap<String, FunctionLocation>>,
    pub calls: Vec<Call>,
    pub type_signatures: HashMap<String, HashMap<String, FunctionSignature>>,
}

#[derive(Debug, Deserialize)]
pub struct StructDef {
    pub fields: Vec<StructField>,
}

#[derive(Debug, Deserialize)]
pub struct StructField {
    pub default: String,
    pub field: String,
    pub required: bool,
    pub inferred_type: Option<String>,
}

/// Function location with clause-level detail.
///
/// The new format stores each function clause as a separate entry keyed by `function/arity:line`.
/// Fields `name` and `arity` are parsed from the key during deserialization.
#[derive(Debug, Deserialize)]
pub struct FunctionLocation {
    pub file: String,
    #[serde(rename = "source_file")]
    pub source_file: Option<String>,
    #[serde(rename = "source_file_absolute")]
    pub source_file_absolute: Option<String>,
    pub column: Option<u32>,
    pub kind: String,
    pub line: u32,
    pub start_line: u32,
    pub end_line: u32,
    pub pattern: Option<String>,
    pub guard: Option<String>,
    pub source_sha: Option<String>,
    pub ast_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Call {
    pub caller: Caller,
    pub callee: Callee,
    #[serde(rename = "type")]
    pub call_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Caller {
    pub module: String,
    pub function: Option<String>,
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Callee {
    pub module: String,
    pub function: String,
    pub arity: u32,
}

#[derive(Debug, Deserialize)]
pub struct FunctionSignature {
    pub arity: u32,
    pub name: String,
    pub clauses: Vec<Clause>,
}

#[derive(Debug, Deserialize)]
pub struct Clause {
    #[serde(rename = "return")]
    pub return_type: String,
    pub args: Vec<String>,
}
