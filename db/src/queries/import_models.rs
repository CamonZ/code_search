//! JSON import structures for call graph data.
//!
//! These types are used to deserialize the JSON output from the Elixir
//! call graph extractor during the import process.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CallGraph {
    pub structs: HashMap<String, StructDef>,
    pub function_locations: HashMap<String, HashMap<String, FunctionLocation>>,
    pub calls: Vec<Call>,
    #[serde(default)]
    pub specs: HashMap<String, Vec<Spec>>,
    #[serde(default)]
    pub types: HashMap<String, Vec<TypeDef>>,
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
/// Fields `name` and `arity` are deserialized directly from the JSON.
#[derive(Debug, Deserialize)]
pub struct FunctionLocation {
    pub name: String,
    pub arity: u32,
    /// Relative file path (accepts either "file" or "source_file" from JSON)
    #[serde(alias = "source_file")]
    pub file: Option<String>,
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
    #[serde(default = "default_complexity")]
    pub complexity: u32,
    #[serde(default)]
    pub max_nesting_depth: u32,
    #[serde(default)]
    pub generated_by: Option<String>,
    #[serde(default)]
    pub macro_source: Option<String>,
}

fn default_complexity() -> u32 {
    1
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
    /// Function kind: "def", "defp", "defmacro", "defmacrop"
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Callee {
    pub module: String,
    pub function: String,
    pub arity: u32,
    /// Argument names (comma-separated in source)
    pub args: Option<String>,
}

/// A @spec or @callback definition.
///
/// Format from extracted_trace.json:
/// ```json
/// {
///   "arity": 1,
///   "line": 19,
///   "name": "function_name",
///   "kind": "spec",
///   "clauses": [{ "full": "...", "inputs_string": [...], "return_string": "..." }]
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct Spec {
    pub name: String,
    pub arity: u32,
    pub line: u32,
    pub kind: String,
    pub clauses: Vec<SpecClause>,
}

/// A single clause within a spec definition.
#[derive(Debug, Deserialize)]
pub struct SpecClause {
    pub full: String,
    pub input_strings: Vec<String>,
    pub return_strings: Vec<String>,
}

/// A @type, @typep, or @opaque definition.
///
/// Format from extracted_trace.json:
/// ```json
/// {
///   "line": 341,
///   "name": "socket_ref",
///   "params": [],
///   "kind": "type",
///   "definition": "@type socket_ref() :: {Pid, module(), binary(), binary(), binary()}"
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub params: Vec<String>,
    pub definition: String,
}
