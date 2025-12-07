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

#[derive(Debug, Deserialize)]
pub struct FunctionLocation {
    pub arity: u32,
    pub name: String,
    pub file: String,
    pub column: Option<u32>,
    pub kind: String,
    pub end_line: u32,
    pub start_line: u32,
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
